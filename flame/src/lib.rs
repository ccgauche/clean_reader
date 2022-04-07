#![allow(unused)]

//! Here's an example of how to use some of FLAMEs APIs:
//!
//! ```
//! extern crate flame;
//!
//! use std::fs::File;
//!
//! pub fn main() {
//!     // Manual `start` and `end`
//!     flame::start("read file");
//!     let x = read_a_file();
//!     flame::end("read file");
//!
//!     // Time the execution of a closure.  (the result of the closure is returned)
//!     let y = flame::span_of("database query", || query_database());
//!
//!     // Time the execution of a block by creating a guard.
//!     let z = {
//!         let _guard = flame::start_guard("cpu-heavy calculation");
//!         cpu_heavy_operations_1();
//!         // Notes can be used to annotate a particular instant in time.
//!         flame::note("something interesting happened", None);
//!         cpu_heavy_operations_2()
//!     };
//!
//!     // Dump the report to disk
//!     flame::dump_html(&mut File::create("flame-graph.html").unwrap()).unwrap();
//!
//!     // Or read and process the data yourself!
//!     let spans = flame::spans();
//!
//!     println!("{} {} {}", x, y, z);
//! }
//!
//! # fn read_a_file() -> bool { true }
//! # fn query_database() -> bool { true }
//! # fn cpu_heavy_operations_1() {}
//! # fn cpu_heavy_operations_2() -> bool { true }
//! ```

#[macro_use]
extern crate lazy_static;
extern crate thread_id;

mod html;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::io::{Error as IoError, Write};
use std::iter::Peekable;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub type StrCow = Cow<'static, str>;

/* lazy_static! {
    static ref ALL_THREADS: Mutex<Vec<(usize, Option<String>, PrivateFrame)>> =
        Mutex::new(Vec::new());
} */

#[derive(Debug, Clone)]
pub struct Library {
    name: Option<String>,
    current: PrivateFrame,
    epoch: Instant,
}

impl Library {
    pub fn start(&mut self, name: impl Into<StrCow>) {
        let collector = &mut self.current;
        let id = collector.next_id;
        collector.next_id += 1;
        collector.all.push(Event {
            id,
            parent: collector.id_stack.last().cloned(),
            name: name.into(),
            collapse: false,
            start_ns: ns_since_epoch(self.epoch),
            end_ns: None,
            delta: None,
        });
        collector.id_stack.push(id);
    }
    pub fn end(&mut self, name: impl Into<StrCow>) -> u64 {
        end_impl(self, name, false)
    }
    pub fn dump(&self, file: impl Write) -> std::io::Result<()> {
        dump_html(self, file)
    }

    pub fn spans(&self) -> Vec<Span> {
        if ::std::thread::panicking() {
            vec![]
        } else {
            convert_events_to_span(self.current.all.iter())
        }
    }
}

#[derive(Debug, Clone)]
struct PrivateFrame {
    next_id: u32,
    all: Vec<Event>,
    id_stack: Vec<u32>,
}

#[derive(Debug, Clone)]
struct Event {
    id: u32,
    parent: Option<u32>,
    name: StrCow,
    collapse: bool,
    start_ns: u64,
    end_ns: Option<u64>,
    delta: Option<u64>,
}

/// A named timespan.
///
/// The span is the most important feature of Flame.  It denotes
/// a chunk of time that is important to you.
///
/// The Span records
/// * Start and stop time
/// * A list of children (also called sub-spans)
/// * A list of notes
#[derive(Debug, Clone)]
pub struct Span {
    /// The name of the span
    pub name: StrCow,
    /// The timestamp of the start of the span
    pub start_ns: u64,
    /// The timestamp of the end of the span
    pub end_ns: u64,
    /// The time that ellapsed between start_ns and end_ns
    pub delta: u64,
    /// How deep this span is in the tree
    pub depth: u16,
    /// A list of spans that occurred inside this one
    pub children: Vec<Span>,
    #[cfg_attr(feature = "json", serde(skip_serializing))]
    collapsable: bool,
}

/// A note for use in debugging.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Note {
    /// A short name describing what happened at some instant in time
    pub name: StrCow,
    /// A longer description
    pub description: Option<StrCow>,
    /// The time that the note was added
    pub instant: u64,
}

fn ns_since_epoch(epoch: Instant) -> u64 {
    let elapsed = epoch.elapsed();
    elapsed.as_secs() * 1000_000_000 + u64::from(elapsed.subsec_nanos())
}

fn convert_events_to_span<'a, I>(events: I) -> Vec<Span>
where
    I: Iterator<Item = &'a Event>,
{
    let mut iterator = events.peekable();
    let mut v = vec![];
    while let Some(event) = iterator.next() {
        if let Some(span) = event_to_span(event, &mut iterator, 0) {
            v.push(span);
        }
    }
    v
}

fn event_to_span<'a, I: Iterator<Item = &'a Event>>(
    event: &Event,
    events: &mut Peekable<I>,
    depth: u16,
) -> Option<Span> {
    if event.end_ns.is_some() && event.delta.is_some() {
        let mut span = Span {
            name: event.name.clone(),
            start_ns: event.start_ns,
            end_ns: event.end_ns.unwrap(),
            delta: event.delta.unwrap(),
            depth,
            children: vec![],
            collapsable: event.collapse,
        };

        loop {
            {
                match events.peek() {
                    Some(next) if next.parent != Some(event.id) => break,
                    None => break,
                    _ => {}
                }
            }

            let next = events.next().unwrap();
            let child = event_to_span(next, events, depth + 1);
            if let Some(child) = child {
                // Try to collapse with the previous span
                if !span.children.is_empty() && child.collapsable && child.children.is_empty() {
                    let last = span.children.last_mut().unwrap();
                    if last.name == child.name && last.depth == child.depth {
                        last.end_ns = child.end_ns;
                        last.delta += child.delta;
                        continue;
                    }
                }

                // Otherwise, it's a new node
                span.children.push(child);
            }
        }
        Some(span)
    } else {
        None
    }
}

impl Library {
    pub fn new() -> Library {
        Library {
            name: ::std::thread::current().name().map(Into::into),
            current: PrivateFrame {
                all: vec![],
                id_stack: vec![],
                next_id: 0,
            },
            epoch: Instant::now(),
        }
    }
}

/// Starts and ends a `Span` that lasts for the duration of the
/// function `f`.
pub fn span_of<S, F, R>(library: &mut Library, name: S, f: F) -> R
where
    S: Into<StrCow>,
    F: FnOnce() -> R,
{
    let name = name.into();
    library.start(name.clone());
    let r = f();
    library.end(name);
    r
}

fn end_impl<S: Into<StrCow>>(library: &mut Library, name: S, collapse: bool) -> u64 {
    use std::thread;

    let name = name.into();
    let delta = {
        let epoch = library.epoch;
        let collector = &mut library.current;

        let current_id = match collector.id_stack.pop() {
            Some(id) => id,
            None if thread::panicking() => 0,
            None => panic!(
                "flame::end({:?}) called without a currently running span!",
                &name
            ),
        };

        let event = &mut collector.all[current_id as usize];

        if event.name != name {
            panic!("flame::end({}) attempted to end {}", &name, event.name);
        }

        let timestamp = ns_since_epoch(epoch);
        event.end_ns = Some(timestamp);
        event.collapse = collapse;
        event.delta = Some(timestamp - event.start_ns);
        event.delta
    };

    match delta {
        Some(d) => d,
        None => 0, // panicking
    }
}

pub use html::{dump_html, dump_html_custom};

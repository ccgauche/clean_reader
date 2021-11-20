# Clean Reader

## What's clean reader?

Clean Reader is an open-source server-side reader mode written in Rust.
Main goals of clean reader:

- Reduce data usage and make the web greener (92% less data usage on average)
- Extract only meaningful content
- Remove all web bloat (JS, Fonts, CSS, Ads...)
- Make pages static and tracker free
- Make pages uniform, customizable, and easy to read
- Make pages load faster (With no JS, CSS... pages load a lot faster).
- Make pages downloadable in one small static HTML file
- Reduce web tracking from your ISP, Government, and companies like Google, Facebook...

## How does it work?

- A server fetches the page you want to access for you (Only the HTML part)
- It extracts the main content of the page
- It generates a clean tree structure from it
- And generate back a tiny HTML file
- Cache it to reduce data consumption

## Some numbers

- As of September 2016, the average web page is 2496 kB in size and requires 140 requests.
- The average clean reader page is 14kB in size and requires one request.

![https://i.imgur.com/QdrBGGO.png](https://i.imgur.com/QdrBGGO.png)

## Installation

### User

#### Google chrome / Firefox

Download the extension and load it in google chrome

#### Hosting your own instance

Go in `releases` and download the latest server for your OS

### Developer

#### Compile from source

- Clone the repo
- Run: `cargo build --release`
- The server can be found in `target/release`

#### Switch to your own server in the extension

- Open the extension panel
- Enter the IP of your server in the server field (the default server URL will be `http://localhost:8080`)

## Contribute

All contributions are welcome! Feel free to fork, open issues, create PRs and go on discord.

## Special thanks

Icons made by [Freepik](https://www.freepik.com) from [www.flaticon.com](https://www.flaticon.com/)

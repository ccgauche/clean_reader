
chrome.contextMenus.create({
  title: "Clean Read", 
  contexts:["link"], 
  onclick: ((info,_b) => {
      chrome.tabs.create({  
        url: "http://localhost:8080/r/"+btoa(info.linkUrl).split("/").join("_")
      });
  })
});
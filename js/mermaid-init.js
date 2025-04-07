document.addEventListener("DOMContentLoaded", function () {
  mermaid.initialize({
    startOnLoad: true,
    theme: "dark",
    securityLevel: "loose",
    flowchart: {
      curve: 'basis',
      defaultRenderer: 'dagre-d3'
    },
    themeVariables: {
      primaryColor: "#42b983",
      primaryTextColor: "#fff",
      primaryBorderColor: "#42b983",
      lineColor: "#aaa",
      secondaryColor: "#304455",
      tertiaryColor: "#111",
      mainBkg: "#1e1e1e",
      secondBkg: "#252526",
      textColor: "#eee",
      nodeBorder: "#42b983",
      clusterBkg: "#1e1e1e"
    }
  });
});

export function svgUse(id, box) {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", box || (id === "i-chevron" || id === "i-chevron-left" ? "0 0 16 16" : "0 0 24 24"));
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("href", "#" + id);
  svg.appendChild(use);
  return svg;
}

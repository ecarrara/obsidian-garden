import * as d3 from "https://cdn.skypack.dev/d3@7"
import { forceSimulation } from "https://cdn.skypack.dev/d3-force@3"
import { drag } from "https://cdn.skypack.dev/d3-drag@3"

export const toc = (articleEl, tocSectionEl) => {
  let toc = document.createElement("ul")

  const createListItem = (text, href) => {
    const listItem = document.createElement("li")
    const a = document.createElement("a")
    a.innerText = text
    a.setAttribute("href", href)
    listItem.append(a)
    return listItem
  }

  let prev = null
  let parent = null

  for (const current of articleEl.querySelectorAll("h1, h2, h3, h4")) {
    if (current.classList.contains('note-title')) {
      continue
    }

    const level = parseInt(current.nodeName[1])
    const text = current.innerText

    current.setAttribute("name", text)
    current.setAttribute("id", text)
    
    if (prev === null) {
      parent = toc
    } else if (level === prev.level) {
      parent = prev.parentElement
    } else if (level > prev.level) {
      const newLevel = document.createElement("ul")
      newLevel.level = level
      prev.appendChild(newLevel)
      parent = newLevel
    } else if (level < prev.level) {
      parent = prev.parentElement
      while (parent.level > level - 1) {
        parent = parent.parentElement
      }
    }

    const newItem = createListItem(current.innerText, `#${text}`)
    newItem.level = level
    parent.appendChild(newItem)
    prev = newItem
  }

  if (toc.children.length > 0) {
    tocSectionEl.classList.remove("hide")
    tocSectionEl.appendChild(toc)
  }
}

export const initGraph = (currentPath, graph) => {
  const w = 220
  const h = 220
  const svg = d3.create("svg")
    .attr("width", w)
    .attr("height", h)

  const links_el = svg.append("g").attr("class", "links")
  const nodes_el = svg.append("g").attr("class", "nodes")


  const nodes = graph.nodes.map((node, index) => ({
    index: index,
    path: node,
    current: node == currentPath,
    x: Math.random(),
    y: Math.random(),
    fx: node == currentPath ? w / 2 : null,
    fy: node == currentPath ? h / 2 : null,
    radius: node == currentPath ? 14 : 6,
  }))

  const links = graph.edges.map((edge) => ({
    source: edge[0],
    target: edge[1]
  }))

  const simulation = forceSimulation(nodes)
    .force("x", d3.forceX(w / 2))
    .force("y", d3.forceY(h / 2))
    .force("charge", d3.forceManyBody().strength(-150))
    .force("center", d3.forceCenter(w / 2, h / 2))
    .force("collision", d3.forceCollide().radius(d => d.radius))
    .force("link", d3.forceLink().links(links).distance(100))

  const dragHandler = drag()
    .on("start", (event) => {
      if (!event.active) simulation.alphaTarget(0.3).restart()
      event.subject.dragged = true
      event.subject.fx = event.subject.x
      event.subject.fy = event.subject.y
    })
    .on("drag", (event) => {
      event.subject.fx = event.x
      event.subject.fy = event.y
    })
    .on("end", (event) => {
      if (!event.active) simulation.alphaTarget(0)
      event.subject.fx = null
      event.subject.fy = null
    })

  const maxPathLength = 32

  const wrap = (text, width) => {
    if (text.length <= width) {
      return text
    }

    const wrapped = []
    let currentLength = 0
    for (const word of text.split(/\s/)) {
      currentLength += word.length + 1
      if (currentLength >= width) {
        break
      }
      wrapped.push(word)
    }
    wrapped.push("…")

    return wrapped.join(" ")
  }

  simulation.on("tick", () => {
    nodes_el
      .selectAll("a")
      .data(nodes)
      .call(dragHandler)
      .join(
        enter => {
          const node = enter.append("a").classed("current", d => d.current).attr("href", d => `/${d.path}.html`)
          node.append("circle").attr("r", d => d.radius)
          node.append("text").text(d => wrap(d.path.split("/").pop(), maxPathLength))
            .attr("dy", d => d.current ? 28: 18)
          node.call(dragHandler)
          return node
        },
        update => update.attr("transform", d => `translate(${d.x}, ${d.y})`)
      )

    links_el
      .selectAll("line")
      .data(links)
      .join("line")
      .attr("x1", d => d.source.x)
      .attr("y1", d => d.source.y)
      .attr("x2", d => d.target.x)
      .attr("y2", d => d.target.y)
  })

  return svg
}
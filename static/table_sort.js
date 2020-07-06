const tableBody = document.getElementById('tableBody')
const thead = tableBody.previousElementSibling
const rows = [...tableBody.rows]
const orders = [1, 1, 1, 1, 1, 1, 1, 1]

const sort = (header, col, type) => {
  let rowCount = rows.length
  rows.sort((a, b) => {
    if (type === 'text') {
      let i = a.children[col].firstChild.nodeValue,
        j = b.children[col].firstChild.nodeValue
      return i === j ? 0 : i > j ? orders[col] : -orders[col]
    } else if (type === 'number') {
      let i = parseInt(a.children[col].firstChild.nodeValue),
        j = parseInt(b.children[col].firstChild.nodeValue)
      return i === j ? 0 : i > j ? orders[col] : -orders[col]
    }
  })
  orders[col] *= -1
  activeHeader(header)
  sortIndicator(header, orders[col])
  while (tableBody.lastChild) tableBody.lastChild.remove()
  while (rowCount--) tableBody.prepend(rows[rowCount])
}

const activeHeader = header => {
  const thArray = Array.prototype.slice.call(thead.querySelectorAll('th'));
  thArray.forEach(th => {
    th.className = ''
    th.removeAttribute('aria-label')
    if (th === header) {
      th.classList.add('active')
    }
  })
}

const sortIndicator = (header, ordering) => {
  if (ordering === 1) {
    header.classList.remove('asc')
    header.classList.add('desc')
    header.setAttribute('aria-label', 'sort by ' + header.innerHTML + ' in descending order')
  } else if (ordering === -1) {
    header.classList.remove('desc')
    header.classList.add('asc')
    header.setAttribute('aria-label', 'sort by ' + header.innerHTML + ' in ascending order')
  }
}

thead.addEventListener(
  'click',
  event => {
    let target = event.target
    let type = target.dataset.type
    if (target.nodeName.toLowerCase() === 'th') {
      sort(target, target.cellIndex, type)
    }
  },
  0
)

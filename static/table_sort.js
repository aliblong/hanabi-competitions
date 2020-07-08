// https://www.smashingmagazine.com/2019/01/table-design-patterns-web/
const tableBody = document.getElementById('tableBody')
const thead = tableBody.previousElementSibling
const rows = [...tableBody.rows]
// remember to update this if the number of columns ever changes
// I didn't write this myself, and it seems kind of smelly, but I can't be  bothered
// to figure out a better way.
const num_headers = Array.prototype.slice.call(thead.querySelectorAll('th')).length;
const orders = [];
for (var i = 0; i < num_headers; i++) {
    orders.push(1);
}

const sort = (header, col, type) => {
  let rowCount = rows.length
  rows.sort((a, b) => {
    if (type === 'text') {
      // I modified this to be much simpler, and now it also works better
      let i = a.children[col].innerText,
        j = b.children[col].innerText
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

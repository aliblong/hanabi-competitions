const frac_mp_entries = document.getElementsByClassName("frac_mp");

for (frac_mp of frac_mp_entries) {
    let text = frac_mp.innerText;
    if (text === "") {
        continue;
    }
    frac_mp.textContent =
        parseFloat(text) === 1
        ? "1"
        : parseFloat(text).toFixed(3).substring(1);
}

const sum_frac_mp_entries = document.getElementsByClassName("sum_frac_mp");

for (sum_frac_mp of sum_frac_mp_entries) {
    sum_frac_mp.innerText = parseFloat(sum_frac_mp.innerText).toFixed(3);
}

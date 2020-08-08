const frac_mp_entries = document.getElementsByClassName("frac_mp");

for (frac_mp of frac_mp_entries) {
    frac_mp.innerHTML =
        parseFloat(frac_mp.innerHTML) === 1
        ? "1"
        : parseFloat(frac_mp.innerHTML).toFixed(3).substring(1);
}

const sum_frac_mp_entries = document.getElementsByClassName("sum_frac_mp");

for (sum_frac_mp of sum_frac_mp_entries) {
    sum_frac_mp.innerHTML = parseFloat(sum_frac_mp.innerHTML).toFixed(3);
}

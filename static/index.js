$("#select_series").simpleSelect({
    defaultSelected: "Choose a series",
    notFoundMessage: "No matching series"
})

$("#select_competition").simpleSelect({
    defaultSelected: "Choose a competition",
    notFoundMessage: "No matching competition"
})

function get_series(form){
    const series_name = $("#select_series").val();
    const base_url = "/series";

    form.action = `${base_url}/${series_name}`;
    form.submit();
}

function get_competition_results_nested(form){
    const competition_name = $("#select_competition").val();
    const base_url = "/competitions";

    form.action = `${base_url}/${competition_name}`;
    form.submit();
}

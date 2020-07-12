$("#select_competition").simpleSelect({
    defaultSelected: "Choose a competition",
    notFoundMessage: "No matching competition"
})

function get_competition_results_nested(form){
    const competition_name = $("#select_competition").val();
    const base_url = "/competitions";

    form.action = `${base_url}/${competition_name}`;
    form.submit();
}

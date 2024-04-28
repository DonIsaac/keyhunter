function foo(a, b) {
    let c = a + b;
    let d = "this is a string that could maybe even be potentially large, but not that large in the grand scheme of things."
    let e = d + " This one also has string concatenation which is crazy"

    if (e) {
        console.log(e)
    } else {
        console.log("e doesn't exist even though it's a constant")
    }

    return e.toLocaleLowerCase().concat(d)
}

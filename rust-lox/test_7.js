var a = "global";

function showA() {
    console.log(a);
}

showA();
if (true) {
    var a = "block";
}
showA();
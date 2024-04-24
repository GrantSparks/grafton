function debounce(func, wait = 20, immediate = true) {
  var timeout;
  return function () {
    var context = this, args = arguments;
    var later = function () {
      timeout = null;
      if (!immediate) func.apply(context, args);
    };
    var callNow = immediate && !timeout;
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
    if (callNow) func.apply(context, args);
  };
}

var h1 = document.querySelector('h1');
var width = window.innerWidth;
var fontSize = width / 10; // adjust this value as needed
document.documentElement.style.setProperty('--font-size', fontSize + 'px');

window.addEventListener('resize', debounce(function () {
  var width = window.innerWidth;
  var fontSize = width / 10; // adjust this value as needed
  document.documentElement.style.setProperty('--font-size', fontSize + 'px');
}));

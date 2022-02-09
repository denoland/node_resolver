let foo = require("foo");
console.log(foo);

let bar = require("foo/bar.js");
console.log(bar);

let cat = require("foo/dir/cat.js");
console.log(cat);

let sibling = require("./sibling");
console.log(sibling);

let sibling2 = require("./sibling.js");
console.log(sibling2);

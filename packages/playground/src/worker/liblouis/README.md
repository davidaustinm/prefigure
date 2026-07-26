The node package for liblouis does not work in a modern browser.
This module consists of `build-no-tables-utf32.js` which comes from
assets directly build from liblouis github (e.g. here https://github.com/liblouis/liblouis/actions/runs/12137141294 
or https://github.com/liblouis/js-build )
Then, at the end of the JS file

```
export { liblouisBuild as capi };
```
is appended. (This makes it importable into Javascript modules.)

The `easy-api` is reimplemented from `easy-api.js` but with major modifications coming from examining the source at http://liblouis.io/translate/

Braille translation tables were fetched from Github: https://github.com/liblouis/js-build/tree/master/tables

# PreFigure Playground

An in-browser playground for PreFigure.

## Building

Before building, you must run `poetry build` in the root `prefigure/` directory in order
to create a `.whl` file (a wheel). This file is dynamically copied when building the playground
so that the playground always uses the development version of PreFigure.

After a prefigure wheel has been built, run

```
npm install
npm run build
```

or, to start in development mode

```
npm install
npm run dev
```

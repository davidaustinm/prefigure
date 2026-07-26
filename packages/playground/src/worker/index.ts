import { expose } from "comlink";
import { PreFigureCompiler } from "./compiler";

const compiler = new PreFigureCompiler();

const add = (a: number, b: number) => a + b;

export const api = {
    compiler, add
}

expose(api);
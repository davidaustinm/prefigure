import { createStore } from "easy-peasy";
import { PlaygroundModel, playgroundModel } from "./model";

export const store = createStore<PlaygroundModel>(playgroundModel);

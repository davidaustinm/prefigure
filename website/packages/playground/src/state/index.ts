import { createTypedHooks } from "easy-peasy";
import { PlaygroundModel } from "./model";

const typedHooks = createTypedHooks<PlaygroundModel>();

export const useStoreActions = typedHooks.useStoreActions;
export const useStoreState = typedHooks.useStoreState;
export const useStoreDispatch = typedHooks.useStoreDispatch;

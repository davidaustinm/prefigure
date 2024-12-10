import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { StoreProvider } from "easy-peasy";
import { store } from "./state/store.ts";

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <StoreProvider store={store}>
            <App />
        </StoreProvider>
    </StrictMode>,
);

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CssVarsProvider, useColorScheme } from "@mui/joy/styles";

function ForceDarkMode() {
    const { setMode } = useColorScheme();

    React.useEffect(() => {
        setMode("dark");
    }, [setMode]);

    return null;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <CssVarsProvider>
            <ForceDarkMode />
            <App />
        </CssVarsProvider>
    </React.StrictMode>,
);

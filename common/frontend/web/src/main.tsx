import React from "react";
import ReactDOM from "react-dom/client";
import Monsters from "./components/Monsters";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Monsters version="NA" usedBy="NA" connectState={false} gui keyGuide guiUpdate netPass net cmdCallback={async () => { return false; }} sendPasswordCallback={async () => { return false; }} />
    </React.StrictMode>
);

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Monsters } from '@monsters/frontend';
import * as Codes from '@monsters/codes';
import "./App.css";
import { listen } from "@tauri-apps/api/event";

function App() {
    const [__debugfix, __setDebugFix] = useState<boolean>(false);
    useEffect(() => {
        if (__debugfix) {
            invoke("frontend_loaded");
        } else {
            __setDebugFix(true);
        }
    }, [__debugfix]);

    const [connectState, setConnectState] = useState<boolean | null>(null);
    const [gui, setGui] = useState(true);
    const [keyGuide, setKeyGuide] = useState(false);
    const [guiUpdate, setGuiUpdate] = useState(false);
    const [net, setNet] = useState(false);
    const [netPass, setNetPass] = useState(false);

    const cmdCallback = async (cmd: number) => {
        if (cmd < 0b10000000) {
            return await invoke<boolean>("cmd", { b: cmd });
        } else {
            let ret = false;
            switch (cmd) {
                case Codes.NET_OFF:
                    setNet(false);
                    ret = await invoke<boolean>("net_off");
                    break;
                case Codes.NET_ON:
                    setNet(true);
                    ret = await invoke<boolean>("net_on");
                    break;
                case Codes.SEND_DATA_OFF:
                    ret = await invoke<boolean>("cmd", { b: cmd });
                    break;
                case Codes.SEND_DATA_ON:
                    ret = await invoke<boolean>("cmd", { b: cmd });
                    break;
                case Codes.KEY_GUIDE_OFF:
                    setKeyGuide(false);
                    ret = true;
                    break;
                case Codes.KEY_GUIDE_ON:
                    setKeyGuide(true);
                    ret = true;
                    break;
                case Codes.INPUT_KEYBOARD:
                    setGui(false);
                    ret = true;
                    break;
                case Codes.INPUT_GUI:
                    setGui(true);
                    ret = true;
                    break;

                case Codes.RESET:
                    ret = await invoke<boolean>("cmd", { b: cmd });
                    break;
            }
            return ret;
        }
    };

    const sendPasswordCallback = async (pass: string) => {
        setNetPass(false);
        return await invoke<boolean>("net_auth", { pass });
    };

    useEffect(() => {
        const unlisten = listen<number>("cmd", ev => {
            const b = ev.payload;

            switch (b) {
                case Codes.SEND_DATA_OFF:
                    setGuiUpdate(false);
                    break;
                case Codes.SEND_DATA_ON:
                    setGuiUpdate(true);
                    break;
                case Codes.AUTHORIZED:
                    setNetPass(false);
                    break;
                case Codes.UNAUTHORIZED:
                    setNetPass(true);
                    break;
                case Codes.DISCONNECTED:
                    setConnectState(null);
                    break;
                case Codes.CONNECTING:
                    setConnectState(false);
                    break;
                case Codes.CONNECTED:
                    setConnectState(true);
                    break;

            }
        });

        return () => {
            unlisten.then(v => v());
        };
    }, []);

    return (
        <Monsters version="0.0.0" usedBy="Tauri" connectState={connectState} gui={gui} keyGuide={keyGuide} guiUpdate={guiUpdate} netPass={netPass} net={net} cmdCallback={cmdCallback} sendPasswordCallback={sendPasswordCallback} />
    );
}

export default App;

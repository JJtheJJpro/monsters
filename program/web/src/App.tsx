import { useState } from 'react';
import { Monsters } from '@monsters/frontend';
import * as Codes from '@monsters/codes';
import './App.css';

function App() {
    const [gui, setGui] = useState(true);
    const [keyGuide, setKeyGuide] = useState(false);
    const [guiUpdate, setGuiUpdate] = useState(false);

    const cmdCallback = async (cmd: number) => {
        if (cmd < 0b10000000) {

        } else {
            switch (cmd) {
                case Codes.SEND_DATA_OFF:
                    break;
                case Codes.SEND_DATA_ON:
                    break;
                case Codes.UNAUTHORIZED:
                    break;
                case Codes.AUTHORIZED:
                    break;
                case Codes.KEY_GUIDE_OFF:
                    break;
                case Codes.KEY_GUIDE_ON:
                    break;
                case Codes.INPUT_KEYBOARD:
                    break;
                case Codes.INPUT_GUI:
                    break;

                case Codes.RESET:
                    break;
            }
        }

        return true;
    };

    return (
        <Monsters version="0.0.0" usedBy="Browser" connected={false} gui={gui} keyGuide={keyGuide} guiUpdate={guiUpdate} cmdCallback={cmdCallback} />
    )
}

export default App

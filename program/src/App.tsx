import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { DialogContent, DialogTitle, Input, Modal, ModalClose, ModalDialog } from "@mui/joy";
import { useColorScheme, CssVarsProvider } from "@mui/joy/styles";

function ForceDarkMode() {
    const { setMode } = useColorScheme();

    useEffect(() => {
        setMode('dark');
    }, [setMode]);

    return null;
}

function App() {
    const [names, setNames] = useState(["Monster 1", "Monster 2", "Monster 3", "Monster 4", "Monster 5", "Monster 6", "Monster 7", "Monster 8", "Monster 9", "Monster 10"]);
    const [nameChange, setNameChange] = useState([false, false, false, false, false, false, false, false, false, false]);
    const [textOnChange, setTextOnChange] = useState("");

    //async function greet() {
    //    setGreetMsg(await invoke("greet", { name }));
    //}

    return (
        <>
            <div>
                <p>Monster Controller</p>
                <p>v0.0.0 Application</p>
            </div>
            <div>
                <table>
                    <thead>
                        <tr>
                            {names.map((n, i) =>
                                <th key={i} onDoubleClick={() => {
                                    let newNameChange = nameChange;
                                    newNameChange[i] = true;
                                    setNameChange([...newNameChange]);
                                }}>
                                    {n}
                                    <CssVarsProvider>
                                        <ForceDarkMode />
                                        <Modal open={nameChange[i]} onClose={() => {
                                            let newNameChange = nameChange;
                                            newNameChange[i] = false;
                                            setNameChange([...newNameChange]);

                                            if (textOnChange != "") {
                                                
                                                setTextOnChange("");
                                            }
                                        }} onKeyDown={(k) => {
                                            if (k.code == "Enter") {
                                                let newNameChange = nameChange;
                                                newNameChange[i] = false;
                                                setNameChange([...newNameChange]);

                                                if (textOnChange != "") {
                                                    
                                                    setTextOnChange("");
                                                }
                                            }
                                        }}>
                                            <ModalDialog size="lg">
                                                <ModalClose />
                                                <DialogTitle>Enter New Name</DialogTitle>
                                                <DialogContent>Enter a new name for {n}:</DialogContent>
                                                <Input autoFocus size="lg" placeholder={n} sx={{
                                                    '--Input-focusedHighlight': 'red !important',
                                                }} onChange={v => setTextOnChange(v.currentTarget.value)} />
                                            </ModalDialog>
                                        </Modal>
                                    </CssVarsProvider>
                                </th>
                            )}
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            {[...Array(10)].map((_, i) =>
                                <td key={i}>
                                    Power
                                </td>
                            )}
                        </tr>
                        <tr>
                            {[...Array(10)].map((_, i) =>
                                <td key={i}>
                                    Activate
                                </td>
                            )}
                        </tr>
                        <tr>
                            {[...Array(10)].map((_, i) =>
                                <td key={i}>
                                    Alternate Activate
                                </td>
                            )}
                        </tr>
                        <tr>
                            <td colSpan={2}>Key Guide</td>
                            <td colSpan={2}>GUI Update</td>
                            <td colSpan={2}>R E S E T</td>
                            <td colSpan={2}>Door</td>
                            <td colSpan={2}>Garbage</td>
                        </tr>
                        <tr>
                            <td colSpan={5}>Keyboard</td>
                            <td colSpan={5}>GUI</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </>
    );

}

export default App;

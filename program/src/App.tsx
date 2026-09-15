import { useEffect, useRef, useState } from "react";
import "./App.css";
import { Button, DialogContent, DialogTitle, Input, Modal, ModalClose, ModalDialog, Sheet, Table } from "@mui/joy";
import { Cmd, cmdToBitIndex, isMonsterDataBitSet, useKeyboardShortcuts } from "./useKeyboardShortcuts";
import { invoke } from "@tauri-apps/api/core";

const UNIFORM_BORDER_SIZE = "2px";

const BLACK = "#000000";
const MAIN_COLOR = "#FF0000";
const DISABLED_COLOR = "#7F0000";

const HOVER_COLOR = "#140000";
const HOVER_ACTIVE_COLOR = "#EC0000";

const ACTIVE_COLOR = "#2A0000";
const ACTIVE_ACTIVE_COLOR = "#D60000";

const DEFAULT_MONSTER_NAMES = [
    "Monster 1", "Monster 2", "Monster 3", "Monster 4", "Monster 5",
    "Monster 6", "Monster 7", "Monster 8", "Monster 9", "Monster 10",
];

type MonsterNameProps = {
    name: string;
    renameOpen: boolean;
    renameDraft: string;
    onOpenRename: () => void;
    onRenameDraftChange: (value: string) => void;
    onRenameCommit: () => void;
    onRenameCancel: () => void;
};

function MonsterName(props: MonsterNameProps) {
    const inputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
        if (!props.renameOpen) {
            return;
        }

        const focusFrame = requestAnimationFrame(() => inputRef.current?.focus());
        return () => cancelAnimationFrame(focusFrame);
    }, [props.renameOpen]);

    const handleClose = () => {
        props.onRenameCancel();
    };

    const handleInputKeyDown: React.KeyboardEventHandler<HTMLInputElement> = (event) => {
        event.stopPropagation();

        if (event.code === "Enter") {
            event.preventDefault();
            props.onRenameCommit();
        } else if (event.code === "Escape") {
            event.preventDefault();
            props.onRenameCancel();
        }
    };

    return (
        <th onClick={props.onOpenRename}>
            <CellButton label={props.name} active={false} variant="solid" />
            <Modal open={props.renameOpen} onClose={handleClose}>
                <ModalDialog size="lg" onClick={e => e.stopPropagation()}>
                    <ModalClose />
                    <DialogTitle>Enter New Name</DialogTitle>
                    <DialogContent>Enter a new name for {props.name}:</DialogContent>
                    <Input
                        autoFocus
                        slotProps={{ input: { ref: inputRef } }}
                        size="lg"
                        value={props.renameDraft}
                        placeholder={props.name}
                        sx={{ "--Input-focusedHighlight": "red !important" }}
                        onChange={e => props.onRenameDraftChange(e.currentTarget.value)}
                        onKeyDown={handleInputKeyDown}
                    />
                    <Button
                        color="danger"
                        size="lg"
                        variant="solid"
                        onClick={props.onRenameCommit}
                    >
                        Change
                    </Button>
                </ModalDialog>
            </Modal>
        </th>
    );
}

function CellButton(props: {
    label: string,
    active: boolean,
    disabled?: boolean,
    variant?: "solid" | "soft" | "outlined",
    onClick?: React.MouseEventHandler<HTMLAnchorElement>
}) {
    const btnStyle = {
        width: "100%",
        height: "100%",
        borderRadius: 0,
        fontFamily: "Cascadia",
        fontSize: "inherit",
        bgcolor: props.active ? MAIN_COLOR : BLACK,
        color: props.active ? BLACK : MAIN_COLOR,
        borderColor: MAIN_COLOR,
        "&:hover": {
            bgcolor: props.active ? HOVER_ACTIVE_COLOR : HOVER_COLOR,
        },
        "&:active": {
            bgcolor: props.active ? ACTIVE_ACTIVE_COLOR : ACTIVE_COLOR,
        },
        "&.Mui-disabled": {
            bgcolor: props.active ? DISABLED_COLOR : BLACK,
            color: props.active ? BLACK : DISABLED_COLOR,
            borderColor: props.active ? MAIN_COLOR : DISABLED_COLOR,
            cursor: "not-allowed",
            pointerEvents: "auto",
        },
        "&.Mui-disabled:hover": {
            bgcolor: props.active ? DISABLED_COLOR : BLACK,
            color: props.active ? BLACK : DISABLED_COLOR,
            borderColor: props.active ? MAIN_COLOR : DISABLED_COLOR,
            cursor: "not-allowed",
            pointerEvents: "auto",
        },
    };

    return (
        <Button variant={props.variant ?? "soft"} color="danger" size="lg" sx={btnStyle} disabled={props.disabled} onClick={props.onClick}>
            {props.label}
        </Button>
    );
}

function App() {
    const [gui, setGui] = useState(true);
    const [monsterNames, setMonsterNames] = useState<string[]>(() => [...DEFAULT_MONSTER_NAMES]);
    const [renamingIndex, setRenamingIndex] = useState<number | null>(null);
    const [renameDraft, setRenameDraft] = useState("");
    const [monsterData, setMonsterData] = useState(0);
    const [keyGuide, setKeyGuide] = useState(false);
    const [guiUpdate, setGuiUpdate] = useState(false);
    const [net, setNet] = useState(false);

    const [__debugfix, __setDebugFix] = useState<boolean>(false);
    useEffect(() => {
        if (__debugfix) {
            invoke("frontend_loaded");
        } else {
            __setDebugFix(true);
        }
    }, [__debugfix]);

    const openRename = (index: number) => {
        setRenamingIndex(index);
        setRenameDraft(monsterNames[index]);
    };

    const commitRename = () => {
        if (renamingIndex === null) {
            return;
        }

        const trimmed = renameDraft.trim();
        if (trimmed !== "") {
            setMonsterNames(prev => prev.map((name, i) =>
                i === renamingIndex ? trimmed : name
            ));
        }

        setRenamingIndex(null);
        setRenameDraft("");
    };

    const cancelRename = () => {
        setRenamingIndex(null);
        setRenameDraft("");
    };

    const setMonsterDataBit = async (bitIndex: number, on: boolean) => {
        invoke<boolean>("key_action", { down: on, cmd: bitIndex }).then(v => {
            if (v) {
                setMonsterData(prev => on ? prev | (1 << bitIndex) : prev & ~(1 << bitIndex));
            }
        });
    };

    const handleCmdKeyDown = async (cmd: Cmd) => {
        const bitIndex = cmdToBitIndex(cmd);
        if (bitIndex !== null) {
            setMonsterDataBit(bitIndex, true);
        }
        //await invoke("key_action", { down: true, cmd: cmd });
    };

    const handleCmdKeyUp = async (cmd: Cmd) => {
        const bitIndex = cmdToBitIndex(cmd);
        if (bitIndex !== null) {
            setMonsterDataBit(bitIndex, false);
        }
        //await invoke("key_action", { down: false, cmd: cmd });
    };

    useKeyboardShortcuts({
        onMonsterRenameKey: openRename,
        onCmdKeyDown: handleCmdKeyDown,
        onCmdKeyUp: handleCmdKeyUp,
    });

    const tableStyle = {
        height: "100%",
        tableLayout: "fixed",
        borderCollapse: "collapse",
        borderSpacing: 0,
        border: `${UNIFORM_BORDER_SIZE} solid ${MAIN_COLOR}`,
        "& th, & td": {
            borderStyle: "solid",
            borderColor: MAIN_COLOR,
            borderWidth: UNIFORM_BORDER_SIZE,
        },
        "& thead tr": {
            height: "20%",
        },
        "& thead th": {
            width: "10%",
            color: MAIN_COLOR,
            bgcolor: BLACK,
            textAlign: "center",
            fontSize: "min(1.5vw, 3vh)",
            p: 0,
            boxShadow: "none",
        },
        "& tbody tr.main-row": {
            height: "18%",
        },
        "& tbody tr.main-row td": {
            width: "10%",
            fontSize: "min(1.5vw, 3vh)",
            p: 0,
        },
        "& tbody tr.bottom-row": {
            height: "13%",
        },
        "& tbody tr.bottom-row td": {
            fontSize: "min(3vw, 6vh)",
            p: 0,
        },
    };
    const sheetStyle = {
        width: "100%",
        height: "100%",
        boxSizing: "border-box",
        bgcolor: BLACK,
        fontFamily: "Cascadia",
    };

    return (
        <>
            <div>
                <p>Monster Controller</p>
                <p>v0.0.0 Application</p>
            </div>
            <div>
                <Sheet variant="plain" sx={sheetStyle}>
                    <Table stickyHeader sx={tableStyle}>
                        <thead>
                            <tr>
                                {monsterNames.map((name, i) =>
                                    <MonsterName
                                        key={i}
                                        name={name}
                                        renameOpen={renamingIndex === i}
                                        renameDraft={renamingIndex === i ? renameDraft : ""}
                                        onOpenRename={() => openRename(i)}
                                        onRenameDraftChange={setRenameDraft}
                                        onRenameCommit={commitRename}
                                        onRenameCancel={cancelRename}
                                    />
                                )}
                            </tr>
                        </thead>
                        <tbody>
                            <tr className="main-row">
                                {[...Array(10)].map((_, i) =>
                                    <td key={i}>
                                        <CellButton label="Power" active={isMonsterDataBitSet(monsterData, i)} variant="solid" disabled={!gui} onClick={async () => await setMonsterDataBit(i, !isMonsterDataBitSet(monsterData, i))} />
                                    </td>
                                )}
                            </tr>
                            <tr className="main-row">
                                {[...Array(10)].map((_, i) =>
                                    <td key={i}>
                                        <CellButton label="Activate" active={isMonsterDataBitSet(monsterData, 10 + i)} variant="solid" disabled={!gui} onClick={async () => await setMonsterDataBit(10 + i, !isMonsterDataBitSet(monsterData, 10 + i))} />
                                    </td>
                                )}
                            </tr>
                            <tr className="main-row">
                                {[...Array(10)].map((_, i) =>
                                    <td key={i}>
                                        <CellButton label="Alternate Activate" active={isMonsterDataBitSet(monsterData, 20 + i)} variant="solid" disabled={!gui} onClick={async () => await setMonsterDataBit(20 + i, !isMonsterDataBitSet(monsterData, 20 + i))} />
                                    </td>
                                )}
                            </tr>
                            <tr className="bottom-row">
                                <td colSpan={2}><CellButton label="Key Guide" active={keyGuide} variant="solid" onClick={() => setKeyGuide(prev => !prev)} /></td>
                                <td colSpan={2}><CellButton label="GUI Update" active={guiUpdate} variant="solid" disabled={!gui} onClick={() => setGuiUpdate(prev => !prev)} /></td>
                                <td colSpan={2}><CellButton label="RESET" active={false} variant="solid" disabled={!gui} onClick={() => setMonsterData(0)} /></td>
                                <td colSpan={2}><CellButton label="Door" active={isMonsterDataBitSet(monsterData, 30)} variant="solid" disabled={!gui} onClick={async () => await setMonsterDataBit(30, !isMonsterDataBitSet(monsterData, 30))} /></td>
                                <td colSpan={2}><CellButton label="Garbage" active={isMonsterDataBitSet(monsterData, 31)} variant="solid" disabled={!gui} onClick={async () => await setMonsterDataBit(31, !isMonsterDataBitSet(monsterData, 31))} /></td>
                            </tr>
                            <tr className="bottom-row">
                                <td colSpan={4}><CellButton label="Keyboard" active={!gui} variant="solid" onClick={() => setGui(false)} /></td>
                                <td colSpan={4}><CellButton label="GUI" active={gui} variant="solid" onClick={() => setGui(true)} /></td>
                                <td colSpan={2}><CellButton label="Network" active={net} variant="solid" onClick={() => setNet(true)} /></td>
                            </tr>
                        </tbody>
                    </Table>
                </Sheet>
            </div>
        </>
    );
}

export default App;

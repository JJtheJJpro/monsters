import React from "react";
import styles from "./Monsters.module.css";
import MonsterName from "./MonsterName";
import CellButton from "./CellButton";
import { Cmd } from "./cmd";
import { cmdToBitIndices, isMonsterDataBitSet, isMonsterDataGroupSet, MONSTER_NUMBERS, monsterColumnToBitIndex, monsterNumberToIndex } from "./MonstCodeMachine";
import { useKeyboardShortcuts } from "./useKeyboardShortcuts";
import { useState } from "react";
import { Button, DialogContent, DialogTitle, Modal, ModalDialog, Option, Input, Select, Sheet, Table } from "@mui/joy";
import { CssVarsProvider, useColorScheme } from "@mui/joy/styles";
import * as Codes from "@monsters/codes";
import * as Consts from "./consts";

export type MonstersProps = {
    version: string,
    usedBy: string,

    connectState: boolean | null,

    gui: boolean,
    keyGuide: boolean,
    guiUpdate: boolean,
    netPass: boolean,
    net?: boolean,
    cmdCallback: (cmd: number) => Promise<boolean>,
    sendPasswordCallback: (pass: string) => Promise<boolean>,
};

export function Monsters(props: MonstersProps) {
    const { version, usedBy, connectState, gui, keyGuide, guiUpdate, netPass, net, cmdCallback, sendPasswordCallback } = props;

    const [monsterData, setMonsterData] = useState<Set<number>>(() => new Set());
    const [monsterNames, setMonsterNames] = useState<string[]>(() => [...Consts.DEFAULT_MONSTER_NAMES]);
    const [renamingIndex, setRenamingIndex] = useState<number | null>(null);
    const [renameDraft, setRenameDraft] = useState("");
    const [doorMonsterNumber, setDoorMonsterNumber] = useState<number | null>(null);
    const [garbageMonsterNumber, setGarbageMonsterNumber] = useState<number | null>(null);
    const [setupComplete, setSetupComplete] = useState(false);
    const [curPass, setCurPass] = useState("");
    const specialMonsterNumbers = { door: doorMonsterNumber, garbage: garbageMonsterNumber };
    const doorBitIndices = cmdToBitIndices(Cmd.D, specialMonsterNumbers);
    const garbageBitIndices = cmdToBitIndices(Cmd.G, specialMonsterNumbers);

    const tryCmdCallback = async (cmd: number) => {
        try {
            const sent = await cmdCallback(cmd);
            if (!sent) {
                console.error(`Command ${cmd} was not sent.`);
            } else if (cmd == Codes.RESET) {
                setMonsterData(new Set());
            }
        } catch (error) {
            console.error(`Failed to send cmd ${cmd}.`, error);
        }
    };

    const setMonsterDataBit = async (relayIndex: number, on: boolean) => {
        if (connectState) {
            try {
                const sent = await cmdCallback(relayIndex | (on ? 0b01000000 : 0));
                if (!sent) {
                    console.error(`Relay command for index ${relayIndex} was not sent.`);
                } else {
                    setMonsterData(prev => {
                        const next = new Set(prev);
                        if (on) {
                            next.add(relayIndex);
                        } else {
                            next.delete(relayIndex);
                        }
                        return next;
                    });
                }
            } catch (error) {
                console.error(`Failed to send relay command for index ${relayIndex}.`, error);
            }
        }
    };

    const openRename = (index: number) => {
        if (!setupComplete) {
            return;
        }

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

    const handleCmdKeyDown = async (cmd: Cmd) => {
        if (setupComplete) {
            if (cmd > Cmd.D) {
                switch (cmd) {
                    case Cmd.KeyGuide:
                        await tryCmdCallback(keyGuide ? Codes.KEY_GUIDE_OFF : Codes.KEY_GUIDE_ON);
                        break;
                    case Cmd.GuiUpdate:
                        await tryCmdCallback(guiUpdate ? Codes.SEND_DATA_OFF : Codes.SEND_DATA_ON);
                        break;
                    case Cmd.InputToggle:
                        await tryCmdCallback(gui ? Codes.INPUT_KEYBOARD : Codes.INPUT_GUI);
                        break;
                    case Cmd.NetToggle:
                        if (net !== undefined) {
                            await tryCmdCallback(net ? Codes.NET_OFF : Codes.NET_ON);
                        }
                        break;
                    case Cmd.Reset:
                        await tryCmdCallback(Codes.RESET);
                        break;
                }
            } else {
                if (cmd === Cmd.D) {
                    await toggleCommandBits(doorBitIndices);
                } else {
                    await setCommandBits(cmdToBitIndices(cmd, specialMonsterNumbers), true);
                }
            }
        }
    };

    const handleCmdKeyUp = async (cmd: Cmd) => {
        if (setupComplete && cmd < Cmd.D) {
            await setCommandBits(cmdToBitIndices(cmd, specialMonsterNumbers), false);
        }
    };

    const updateMonsterNumber = (
        value: number | null,
        update: (monsterNumber: number | null) => void,
    ) => {
        if (value === null || monsterNumberToIndex(value) !== null) {
            update(value);
        }
    };

    const setCommandBits = async (bitIndices: number[], on: boolean) => {
        await Promise.all(bitIndices.map(bitIndex => setMonsterDataBit(bitIndex, on)));
    };

    const toggleCommandBits = async (bitIndices: number[]) => {
        await setCommandBits(bitIndices, !isMonsterDataGroupSet(monsterData, bitIndices));
    };

    useKeyboardShortcuts({
        onMonsterRenameKey: openRename,
        onCmdKeyDown: handleCmdKeyDown,
        onCmdKeyUp: handleCmdKeyUp,
    }, connectState);

    const tableStyle = {
        height: "100%",
        tableLayout: "fixed",
        borderCollapse: "collapse",
        borderSpacing: 0,
        border: `${Consts.UNIFORM_BORDER_SIZE} solid ${Consts.MAIN_COLOR}`,
        "& th, & td": {
            borderStyle: "solid",
            borderColor: Consts.MAIN_COLOR,
            borderWidth: Consts.UNIFORM_BORDER_SIZE,
        },
        "& thead tr": {
            height: "20%",
        },
        "& thead th": {
            width: "10%",
            color: Consts.MAIN_COLOR,
            bgcolor: Consts.BLACK,
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
        bgcolor: Consts.BLACK,
        fontFamily: "Doors",
    };

    function ForceDarkMode() {
        const { setMode } = useColorScheme();

        React.useEffect(() => {
            setMode("dark");
        }, [setMode]);

        return null;
    }

    return (
        <CssVarsProvider>
            <ForceDarkMode />
            <div className={styles.root}>
                <Modal open={!setupComplete} onClose={() => undefined}>
                    <ModalDialog size="lg">
                        <DialogTitle>Set Up Special Commands</DialogTitle>
                        <DialogContent>
                            Choose a different relay set (1-12) for each special command: Door and Garbage.
                        </DialogContent>
                        <Select
                            aria-label="Door monster number"
                            placeholder="Select a number for Door"
                            value={doorMonsterNumber}
                            onChange={(_, value) => updateMonsterNumber(value, setDoorMonsterNumber)}
                        >
                            {MONSTER_NUMBERS.map(monsterNumber =>
                                <Option key={monsterNumber} value={monsterNumber} disabled={monsterNumber === garbageMonsterNumber}>{monsterNumber}</Option>
                            )}
                        </Select>
                        <Select
                            aria-label="Garbage monster number"
                            placeholder="Select a number for Garbage"
                            value={garbageMonsterNumber}
                            onChange={(_, value) => updateMonsterNumber(value, setGarbageMonsterNumber)}
                        >
                            {MONSTER_NUMBERS.map(monsterNumber =>
                                <Option key={monsterNumber} value={monsterNumber} disabled={monsterNumber === doorMonsterNumber}>{monsterNumber}</Option>
                            )}
                        </Select>
                        <Button
                            disabled={
                                monsterNumberToIndex(doorMonsterNumber ?? 0) === null
                                || monsterNumberToIndex(garbageMonsterNumber ?? 0) === null
                                || doorMonsterNumber === garbageMonsterNumber
                            }
                            onClick={() => setSetupComplete(true)}
                        >
                            Continue
                        </Button>
                    </ModalDialog>
                </Modal>
                <Modal open={netPass} onClose={() => undefined}>
                    <ModalDialog size="lg">
                        <DialogTitle>Enter Password</DialogTitle>
                        <DialogContent>
                            Enter the password for the monster box:
                        </DialogContent>
                        <Input
                            autoFocus
                            size="lg"
                            //value={props.renameDraft}
                            sx={{ "--Input-focusedHighlight": "red !important" }}
                            onChange={e => setCurPass(e.currentTarget.value)}
                        />
                        <Button
                            color="danger"
                            size="lg"
                            variant="solid"
                            onClick={() => sendPasswordCallback(curPass)}
                        >
                            Authorize
                        </Button>
                    </ModalDialog>
                </Modal>
                <div className={styles.header}>
                    <p className={styles.title}>Monster Controller</p>
                    <p className={styles.subtitle}>v{version} {usedBy}</p>
                    <p className={styles.subtitle}>{connectState ? "Connected" : connectState === false ? "Connecting..." : "Not connected"}</p>
                </div>
                <div className={styles.content}>
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
                                    {[...Array(10)].map((_, i) => {
                                        const bitIndex = monsterColumnToBitIndex(i, "power", specialMonsterNumbers);
                                        if (bitIndex === null) return null;
                                        return <td key={i}>
                                            <CellButton label="Power" active={isMonsterDataBitSet(monsterData, bitIndex)} variant="solid" disabled={!connectState || !gui} onClick={async () => await setMonsterDataBit(bitIndex, !isMonsterDataBitSet(monsterData, bitIndex))} />
                                        </td>
                                    })}
                                </tr>
                                <tr className="main-row">
                                    {[...Array(10)].map((_, i) => {
                                        const bitIndex = monsterColumnToBitIndex(i, "activate", specialMonsterNumbers);
                                        if (bitIndex === null) return null;
                                        return <td key={i}>
                                            <CellButton label="Activate" active={isMonsterDataBitSet(monsterData, bitIndex)} variant="solid" disabled={!connectState || !gui} onClick={async () => await setMonsterDataBit(bitIndex, !isMonsterDataBitSet(monsterData, bitIndex))} />
                                        </td>
                                    })}
                                </tr>
                                <tr className="main-row">
                                    {[...Array(10)].map((_, i) => {
                                        const bitIndex = monsterColumnToBitIndex(i, "alternateActivate", specialMonsterNumbers);
                                        if (bitIndex === null) return null;
                                        return <td key={i}>
                                            <CellButton label="Alternate Activate" active={isMonsterDataBitSet(monsterData, bitIndex)} variant="solid" disabled={!connectState || !gui} onClick={async () => await setMonsterDataBit(bitIndex, !isMonsterDataBitSet(monsterData, bitIndex))} />
                                        </td>
                                    })}
                                </tr>
                                <tr className="bottom-row">
                                    <td colSpan={2}><CellButton label="Key Guide" active={keyGuide} variant="solid" onClick={() => tryCmdCallback(keyGuide ? Codes.KEY_GUIDE_OFF : Codes.KEY_GUIDE_ON)} /></td>
                                    <td colSpan={2}><CellButton label="GUI Update" active={guiUpdate} variant="solid" disabled={!connectState} onClick={() => tryCmdCallback(guiUpdate ? Codes.SEND_DATA_OFF : Codes.SEND_DATA_ON)} /></td>
                                    <td colSpan={2}><CellButton label="RESET" active={false} variant="solid" disabled={!connectState || !gui} onClick={() => tryCmdCallback(Codes.RESET)} /></td>
                                    <td colSpan={2}><CellButton label={doorMonsterNumber === null ? "Door" : `Door (${doorMonsterNumber})`} active={isMonsterDataGroupSet(monsterData, doorBitIndices)} variant="solid" disabled={!connectState || !gui || doorBitIndices.length === 0} onClick={() => toggleCommandBits(doorBitIndices)} /></td>
                                    <td colSpan={2}><CellButton label={garbageMonsterNumber === null ? "Garbage" : `Garbage (${garbageMonsterNumber})`} active={isMonsterDataGroupSet(monsterData, garbageBitIndices)} variant="solid" disabled={!connectState || !gui || garbageBitIndices.length === 0} onClick={() => toggleCommandBits(garbageBitIndices)} /></td>
                                </tr>
                                <tr className="bottom-row">
                                    <td colSpan={net !== undefined ? 4 : 5}><CellButton label="Keyboard" active={!gui} variant="solid" onClick={() => tryCmdCallback(Codes.INPUT_KEYBOARD)} /></td>
                                    <td colSpan={net !== undefined ? 4 : 5}><CellButton label="GUI" active={gui} variant="solid" onClick={() => tryCmdCallback(Codes.INPUT_GUI)} /></td>
                                    {net !== undefined &&
                                        <td colSpan={2}><CellButton label="Network" active={net} variant="solid" onClick={() => tryCmdCallback(net ? Codes.NET_OFF : Codes.NET_ON)} /></td>
                                    }
                                </tr>
                            </tbody>
                        </Table>
                    </Sheet>
                </div>
            </div>
        </CssVarsProvider>

    )
}

export default Monsters;

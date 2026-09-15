import { useEffect, useRef } from "react";

export enum Cmd {
    P0, P1, P2, P3, P4, P5, P6, P7, P8, P9,
    A0, A1, A2, A3, A4, A5, A6, A7, A8, A9,
    X0, X1, X2, X3, X4, X5, X6, X7, X8, X9,

    G,
    D,

    Unknown,
}

export function keyCodeToCmd(code: string): Cmd {
    switch (code) {
        case "KeyQ": return Cmd.P0;
        case "KeyW": return Cmd.P1;
        case "KeyE": return Cmd.P2;
        case "KeyR": return Cmd.P3;
        case "KeyT": return Cmd.P4;
        case "KeyY": return Cmd.P5;
        case "KeyU": return Cmd.P6;
        case "KeyI": return Cmd.P7;
        case "KeyO": return Cmd.P8;
        case "KeyP": return Cmd.P9;

        case "KeyA": return Cmd.A0;
        case "KeyS": return Cmd.A1;
        case "KeyD": return Cmd.A2;
        case "KeyF": return Cmd.A3;
        case "KeyG": return Cmd.A4;
        case "KeyH": return Cmd.A5;
        case "KeyJ": return Cmd.A6;
        case "KeyK": return Cmd.A7;
        case "KeyL": return Cmd.A8;
        case "Semicolon": return Cmd.A9;

        case "KeyZ": return Cmd.X0;
        case "KeyX": return Cmd.X1;
        case "KeyC": return Cmd.X2;
        case "KeyV": return Cmd.X3;
        case "KeyB": return Cmd.X4;
        case "KeyN": return Cmd.X5;
        case "KeyM": return Cmd.X6;
        case "Comma": return Cmd.X7;
        case "Period": return Cmd.X8;
        case "Slash": return Cmd.X9;

        case "Backslash": return Cmd.G;
        case "Space": return Cmd.D;

        default: return Cmd.Unknown;
    }
}

/** Maps a control command to its bit index in monsterData (0–31). */
export function cmdToBitIndex(cmd: Cmd): number | null {
    if (cmd >= Cmd.P0 && cmd <= Cmd.P9) {
        return cmd - Cmd.P0;
    }
    if (cmd >= Cmd.A0 && cmd <= Cmd.A9) {
        return 10 + (cmd - Cmd.A0);
    }
    if (cmd >= Cmd.X0 && cmd <= Cmd.X9) {
        return 20 + (cmd - Cmd.X0);
    }
    if (cmd === Cmd.D) {
        return 30;
    }
    if (cmd === Cmd.G) {
        return 31;
    }
    return null;
}

export function isMonsterDataBitSet(data: number, bitIndex: number): boolean {
    return (data & (1 << bitIndex)) !== 0;
}

function isTypingInField(): boolean {
    const active = document.activeElement;
    return active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement;
}

function digitCodeToMonsterIndex(code: string): number | null {
    if (!code.startsWith("Digit")) {
        return null;
    }
    const digit = Number.parseInt(code.replace("Digit", ""), 10);
    return digit === 0 ? 9 : digit - 1;
}

type KeyboardShortcutHandlers = {
    onMonsterRenameKey?: (monsterIndex: number) => void;
    onCmdKeyDown?: (cmd: Cmd) => Promise<void>;
    onCmdKeyUp?: (cmd: Cmd) => Promise<void>;
};

export function useKeyboardShortcuts(handlers: KeyboardShortcutHandlers) {
    const keysDownRef = useRef<Record<string, boolean>>({});
    const handlersRef = useRef(handlers);
    handlersRef.current = handlers;

    useEffect(() => {
        const onKeyDown = async (ev: KeyboardEvent) => {
            if (isTypingInField()) {
                return;
            }

            const monsterIndex = digitCodeToMonsterIndex(ev.code);
            if (monsterIndex !== null) {
                ev.preventDefault();
                ev.stopPropagation();

                if (!keysDownRef.current[ev.code]) {
                    keysDownRef.current[ev.code] = true;
                    handlersRef.current.onMonsterRenameKey?.(monsterIndex);
                }
                return;
            }

            const cmd = keyCodeToCmd(ev.code);
            if (cmd === Cmd.Unknown) {
                return;
            }

            if (!keysDownRef.current[ev.code]) {
                keysDownRef.current[ev.code] = true;
                await handlersRef.current.onCmdKeyDown?.(cmd);
            }
        };

        const onKeyUp = async (ev: KeyboardEvent) => {
            // Always clear held-key state on release, even when an input has
            // focus. Otherwise a digit that opened the rename dialog leaves a
            // stale "down" flag because keyup fired after focus moved to the input.
            keysDownRef.current[ev.code] = false;

            if (isTypingInField()) {
                return;
            }

            const cmd = keyCodeToCmd(ev.code);
            if (cmd === Cmd.Unknown) {
                return;
            }

            await handlersRef.current.onCmdKeyUp?.(cmd);
        };

        window.addEventListener("keydown", onKeyDown, { capture: true });
        window.addEventListener("keyup", onKeyUp, { capture: true });

        return () => {
            window.removeEventListener("keydown", onKeyDown, true);
            window.removeEventListener("keyup", onKeyUp, true);
        };
    }, []);
}

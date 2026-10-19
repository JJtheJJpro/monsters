import { useEffect, useRef } from "react";
import { digitCodeToMonsterIndex, keyCodeToCmd } from "./MonstCodeMachine"
import { Cmd } from "./cmd";

function isTypingInField(): boolean {
    const active = document.activeElement;
    return active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement;
}

type KeyboardShortcutHandlers = {
    onMonsterRenameKey?: (monsterIndex: number) => void;
    onCmdKeyDown?: (cmd: Cmd) => Promise<void>;
    onCmdKeyUp?: (cmd: Cmd) => Promise<void>;
};

export function useKeyboardShortcuts(handlers: KeyboardShortcutHandlers, connected: boolean | null) {
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

            if (connected || cmd === Cmd.KeyGuide || cmd === Cmd.InputToggle || cmd === Cmd.NetToggle) {
                if (!keysDownRef.current[ev.code]) {
                    keysDownRef.current[ev.code] = true;
                    await handlersRef.current.onCmdKeyDown?.(cmd);
                }
            }
        };

        const onKeyUp = async (ev: KeyboardEvent) => {
            const wasDown = keysDownRef.current[ev.code] === true;
            // Always clear held-key state on release, even when an input has
            // focus. Otherwise a digit that opened the rename dialog leaves a
            // stale "down" flag because keyup fired after focus moved to the input.
            keysDownRef.current[ev.code] = false;

            if (!wasDown || isTypingInField()) {
                return;
            }

            const cmd = keyCodeToCmd(ev.code);
            if (cmd === Cmd.Unknown) {
                return;
            }

            if (connected || cmd === Cmd.KeyGuide || cmd === Cmd.InputToggle || cmd === Cmd.NetToggle) {
                await handlersRef.current.onCmdKeyUp?.(cmd);
            }
        };

        window.addEventListener("keydown", onKeyDown, { capture: true });
        window.addEventListener("keyup", onKeyUp, { capture: true });

        return () => {
            window.removeEventListener("keydown", onKeyDown, true);
            window.removeEventListener("keyup", onKeyUp, true);
        };
    }, [connected]);
}

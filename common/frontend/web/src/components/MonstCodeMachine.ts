import { Cmd } from "./cmd";

export type MonsterAction = "power" | "activate" | "alternateActivate";
export type SpecialMonsterNumbers = {
    door: number | null;
    garbage: number | null;
};

export const MONSTER_NUMBERS = Array.from({ length: 12 }, (_, index) => index + 1);
export const NORMAL_MONSTER_COUNT = 10;

const ACTION_BIT_OFFSETS: Record<MonsterAction, number> = {
    power: 0,
    activate: 12,
    alternateActivate: 24,
};

export function monsterNumberToIndex(monsterNumber: number): number | null {
    if (!Number.isInteger(monsterNumber) || monsterNumber < 1 || monsterNumber > MONSTER_NUMBERS.length) {
        return null;
    }
    return monsterNumber - 1;
}

export function monsterNumberToRelayIndices(monsterNumber: number | null | undefined): number[] | null {
    const index = monsterNumber === null || monsterNumber === undefined
        ? null
        : monsterNumberToIndex(monsterNumber);
    if (index === null) {
        return null;
    }
    return [index, index + 12, index + 24];
}

export function availableMonsterNumbers(specialMonsterNumbers?: SpecialMonsterNumbers): number[] {
    const selectedNumbers = [specialMonsterNumbers?.door, specialMonsterNumbers?.garbage]
        .filter((number): number is number => monsterNumberToIndex(number ?? 0) !== null);
    if (selectedNumbers.length === 2 && selectedNumbers[0] === selectedNumbers[1]) {
        return [];
    }
    return MONSTER_NUMBERS.filter(number => !selectedNumbers.includes(number));
}

export function monsterColumnToBitIndex(
    columnIndex: number,
    action: MonsterAction,
    specialMonsterNumbers?: SpecialMonsterNumbers,
): number | null {
    if (!Number.isInteger(columnIndex) || columnIndex < 0 || columnIndex >= NORMAL_MONSTER_COUNT) {
        return null;
    }
    const monsterNumber = availableMonsterNumbers(specialMonsterNumbers)[columnIndex];
    const monsterIndex = monsterNumber === undefined ? null : monsterNumberToIndex(monsterNumber);
    return monsterIndex === null ? null : ACTION_BIT_OFFSETS[action] + monsterIndex;
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

        case "Enter": return Cmd.Reset;
        case "Equal": return Cmd.KeyGuide;
        case "Minus": return Cmd.GuiUpdate;
        case "ArrowLeft": case "ArrowRight": return Cmd.InputToggle;
        case "Backquote": return Cmd.NetToggle;

        default: return Cmd.Unknown;
    }
}

/** Maps keyboard commands to raw relay indices in monsterData. */
export function cmdToBitIndices(cmd: Cmd, specialMonsterNumbers?: SpecialMonsterNumbers): number[] {
    if (cmd >= Cmd.P0 && cmd <= Cmd.P9) {
        const index = monsterColumnToBitIndex(cmd - Cmd.P0, "power", specialMonsterNumbers);
        return index === null ? [] : [index];
    }
    if (cmd >= Cmd.A0 && cmd <= Cmd.A9) {
        const index = monsterColumnToBitIndex(cmd - Cmd.A0, "activate", specialMonsterNumbers);
        return index === null ? [] : [index];
    }
    if (cmd >= Cmd.X0 && cmd <= Cmd.X9) {
        const index = monsterColumnToBitIndex(cmd - Cmd.X0, "alternateActivate", specialMonsterNumbers);
        return index === null ? [] : [index];
    }
    if (cmd === Cmd.D) {
        return monsterNumberToRelayIndices(specialMonsterNumbers?.door) ?? [];
    }
    if (cmd === Cmd.G) {
        return monsterNumberToRelayIndices(specialMonsterNumbers?.garbage) ?? [];
    }
    return [];
}

export function isMonsterDataBitSet(data: ReadonlySet<number>, bitIndex: number): boolean {
    return data.has(bitIndex);
}

export function isMonsterDataGroupSet(data: ReadonlySet<number>, bitIndices: number[]): boolean {
    return bitIndices.length > 0 && bitIndices.every(bitIndex => isMonsterDataBitSet(data, bitIndex));
}

export function digitCodeToMonsterIndex(code: string): number | null {
    if (!code.startsWith("Digit")) {
        return null;
    }
    const digit = Number.parseInt(code.slice("Digit".length), 10);
    const monsterNumber = digit === 0 ? 10 : digit;
    const index = monsterNumberToIndex(monsterNumber);
    return index !== null && index < 10 ? index : null;
}

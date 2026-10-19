export const Cmd = {
    P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9,
    A0: 10, A1: 11, A2: 12, A3: 13, A4: 14, A5: 15, A6: 16, A7: 17, A8: 18, A9: 19,
    X0: 20, X1: 21, X2: 22, X3: 23, X4: 24, X5: 25, X6: 26, X7: 27, X8: 28, X9: 29,

    G: 30,
    D: 31,

    Reset: 32,
    KeyGuide: 33,
    GuiUpdate: 34,
    InputToggle: 35,
    NetToggle: 36,

    Unknown: 37,
} as const;

export type Cmd = (typeof Cmd)[keyof typeof Cmd];

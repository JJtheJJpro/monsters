import { useEffect, useState } from 'react'
import './App.css'
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

function App() {
    const [enabled, setOn] = useState(false);
    const [inputtype, setInput] = useState<(0 | 1 | 2)>(1);
    const [keyguide, setGuide] = useState(false);
    const [datamode, setDatamode] = useState(false);
    const [names, setNames] = useState(["Monster 1", "Monster 2", "Monster 3", "Monster 4", "Monster 5", "Monster 6", "Monster 7", "Monster 8", "Monster 9", "Monster 10"]);
    const [monsts, setMonsts] = useState([
        false, false, false, false, false, false, false, false, false, false, // P
        false, false, false, false, false, false, false, false, false, false, // A
        false, false, false, false, false, false, false, false, false, false, // X
        false, false // D, G
    ]);
    const [keysdown, setKeysdown] = useState<Record<string, boolean>>({});
    const [__debugfix, __setDebugFix] = useState<boolean>(false);

    useEffect(() => {
        if (__debugfix) {
            invoke("frontend_loaded");
        } else {
            __setDebugFix(true);
        }
    }, [__debugfix]);

    useEffect(() => {
        const uKeypress = (e: KeyboardEvent) => {
            if (e.key == '1' || e.key == '2' || e.key == '3' || e.key == '4' || e.key == '5' || e.key == '6' || e.key == '7' || e.key == '8' || e.key == '9' || e.key == '0') {
                let tks = keysdown;
                tks[e.key] = true;
                setKeysdown(tks);
                switch (e.key) {
                    case '1':
                        let s1 = prompt('Input the name of the monster');
                        if (s1 != null) {
                            let tn = names;
                            tn[0] = s1 == '' ? 'Monster 1' : s1;
                            setNames([...tn]);
                        }
                        break;
                    case '2':
                        let s2 = prompt('Input the name of the monster');
                        if (s2 != null) {
                            let tn = names;
                            tn[1] = s2 == '' ? 'Monster 2' : s2;
                            setNames([...tn]);
                        }
                        break;
                    case '3':
                        let s3 = prompt('Input the name of the monster');
                        if (s3 != null) {
                            let tn = names;
                            tn[2] = s3 == '' ? 'Monster 3' : s3;
                            setNames([...tn]);
                        }
                        break;
                    case '4':
                        let s4 = prompt('Input the name of the monster');
                        if (s4 != null) {
                            let tn = names;
                            tn[3] = s4 == '' ? 'Monster 4' : s4;
                            setNames([...tn]);
                        }
                        break;
                    case '5':
                        let s5 = prompt('Input the name of the monster');
                        if (s5 != null) {
                            let tn = names;
                            tn[4] = s5 == '' ? 'Monster 5' : s5;
                            setNames([...tn]);
                        }
                        break;
                    case '6':
                        let s6 = prompt('Input the name of the monster');
                        if (s6 != null) {
                            let tn = names;
                            tn[5] = s6 == '' ? 'Monster 6' : s6;
                            setNames([...tn]);
                        }
                        break;
                    case '7':
                        let s7 = prompt('Input the name of the monster');
                        if (s7 != null) {
                            let tn = names;
                            tn[6] = s7 == '' ? 'Monster 7' : s7;
                            setNames([...tn]);
                        }
                        break;
                    case '8':
                        let s8 = prompt('Input the name of the monster');
                        if (s8 != null) {
                            let tn = names;
                            tn[7] = s8 == '' ? 'Monster 8' : s8;
                            setNames([...tn]);
                        }
                        break;
                    case '9':
                        let s9 = prompt('Input the name of the monster');
                        if (s9 != null) {
                            let tn = names; 5
                            tn[8] = s9 == '' ? 'Monster 9' : s9;
                            setNames([...tn]);
                        }
                        break;
                    case '0':
                        let s0 = prompt('Input the name of the monster');
                        if (s0 != null) {
                            let tn = names;
                            tn[9] = s0 == '' ? 'Monster 10' : s0;
                            setNames([...tn]);
                        }
                        break;
                }
            }
        };
        const uKeydown = (e: KeyboardEvent) => {
            if (!keysdown[e.key]) {
                let tks = keysdown;
                tks[e.key] = true;
                setKeysdown(tks);
                switch (e.key) {
                    case '=':
                        setGuide(!keyguide);
                        break;
                    case '-':
                        if (enabled) {
                            let nv = !datamode;
                            invoke("write", { v: nv ? 0b10000011 : 0b10000010 });
                            setDatamode(nv);
                        }
                        break;
                    case 'Enter':
                        if (enabled) {
                            reset();
                        }
                        break;
                    case 'ArrowLeft':
                        setInput(inputtype == 0 ? 2 : inputtype == 1 ? 0 : 1);
                        break;
                    case 'ArrowRight':
                        setInput(inputtype == 0 ? 1 : inputtype == 1 ? 2 : 0);
                        break;
                }

                if (enabled && inputtype != 2) {
                    switch (e.key) {
                        case 'q':
                            monstparse(0, true);
                            break;
                        case 'w':
                            monstparse(1, true);
                            break;
                        case 'e':
                            monstparse(2, true);
                            break;
                        case 'r':
                            monstparse(3, true);
                            break;
                        case 't':
                            monstparse(4, true);
                            break;
                        case 'y':
                            monstparse(5, true);
                            break;
                        case 'u':
                            monstparse(6, true);
                            break;
                        case 'i':
                            monstparse(7, true);
                            break;
                        case 'o':
                            monstparse(8, true);
                            break;
                        case 'p':
                            monstparse(9, true);
                            break;

                        case 'a':
                            monstparse(10, true);
                            break;
                        case 's':
                            monstparse(11, true);
                            break;
                        case 'd':
                            monstparse(12, true);
                            break;
                        case 'f':
                            monstparse(13, true);
                            break;
                        case 'g':
                            monstparse(14, true);
                            break;
                        case 'h':
                            monstparse(15, true);
                            break;
                        case 'j':
                            monstparse(16, true);
                            break;
                        case 'k':
                            monstparse(17, true);
                            break;
                        case 'l':
                            monstparse(18, true);
                            break;
                        case ';':
                            monstparse(19, true);
                            break;

                        case 'z':
                            monstparse(20, true);
                            break;
                        case 'x':
                            monstparse(21, true);
                            break;
                        case 'c':
                            monstparse(22, true);
                            break;
                        case 'v':
                            monstparse(23, true);
                            break;
                        case 'b':
                            monstparse(24, true);
                            break;
                        case 'n':
                            monstparse(25, true);
                            break;
                        case 'm':
                            monstparse(26, true);
                            break;
                        case ',':
                            monstparse(27, true);
                            break;
                        case '.':
                            monstparse(28, true);
                            break;
                        case '/':
                            monstparse(29, true);
                            break;

                        case ' ':
                            monstparse(30, !monsts[30]);
                            break;
                        case "'":
                            monstparse(31, true);
                            break;
                    }
                }
            }
        };
        const uKeyup = (e: KeyboardEvent) => {
            let tks = keysdown;
            tks[e.key] = false;
            setKeysdown(tks);
            if (enabled && inputtype != 2) {
                switch (e.key) {
                    case 'q':
                        monstparse(0, false);
                        break;
                    case 'w':
                        monstparse(1, false);
                        break;
                    case 'e':
                        monstparse(2, false);
                        break;
                    case 'r':
                        monstparse(3, false);
                        break;
                    case 't':
                        monstparse(4, false);
                        break;
                    case 'y':
                        monstparse(5, false);
                        break;
                    case 'u':
                        monstparse(6, false);
                        break;
                    case 'i':
                        monstparse(7, false);
                        break;
                    case 'o':
                        monstparse(8, false);
                        break;
                    case 'p':
                        monstparse(9, false);
                        break;

                    case 'a':
                        monstparse(10, false);
                        break;
                    case 's':
                        monstparse(11, false);
                        break;
                    case 'd':
                        monstparse(12, false);
                        break;
                    case 'f':
                        monstparse(13, false);
                        break;
                    case 'g':
                        monstparse(14, false);
                        break;
                    case 'h':
                        monstparse(15, false);
                        break;
                    case 'j':
                        monstparse(16, false);
                        break;
                    case 'k':
                        monstparse(17, false);
                        break;
                    case 'l':
                        monstparse(18, false);
                        break;
                    case ';':
                        monstparse(19, false);
                        break;

                    case 'z':
                        monstparse(20, false);
                        break;
                    case 'x':
                        monstparse(21, false);
                        break;
                    case 'c':
                        monstparse(22, false);
                        break;
                    case 'v':
                        monstparse(23, false);
                        break;
                    case 'b':
                        monstparse(24, false);
                        break;
                    case 'n':
                        monstparse(25, false);
                        break;
                    case 'm':
                        monstparse(26, false);
                        break;
                    case ',':
                        monstparse(27, false);
                        break;
                    case '.':
                        monstparse(28, false);
                        break;
                    case '/':
                        monstparse(29, false);
                        break;

                    case "'":
                        monstparse(31, false);
                        break;
                }
            }
        };

        document.addEventListener('keypress', uKeypress);
        document.addEventListener('keydown', uKeydown);
        document.addEventListener('keyup', uKeyup);

        return () => {
            document.removeEventListener('keypress', uKeypress);
            document.removeEventListener('keydown', uKeydown);
            document.removeEventListener('keyup', uKeyup);
        };
    }, [keyguide, datamode, inputtype, monsts, enabled]);

    useEffect(() => {
        let unlisten = listen<Uint8Array>("global", e => {
            console.log(e);
            if (e.payload[0] < 0b10000000) {
                if (!datamode) {
                    console.log("ignoring the fact that i got a monster command even though the gui update is off...");
                }
                let [n, on] = monstunparse(e.payload[0]);
                if (monsts[n] != on) {
                    let m = monsts;
                    m[n] = on;
                    setMonsts([...m]);
                }
            } else {
                switch (e.payload[0]) {
                    case 0b10000000:
                        setOn(false);
                        break;
                    case 0b11111111:
                        setOn(true);
                        break;
                }
            }

        });

        return () => {
            unlisten.then(f => f());
        }
    }, []);

    function monstunparse(b: number): [number, boolean] {
        return [b & 0b00111111, (b & 0b01000000) != 0]
    }
    function monstparse(n: number, on: boolean) {
        if (monsts[n] != on) {
            if (!datamode) {
                let m = monsts;
                m[n] = on;
                setMonsts([...m]);
            }
            console.log(on ? 0b01000000 | n + 1 : n + 1)
            invoke("write", { v: on ? 0b01000000 | n + 1 : n + 1 })
        }
    }

    function reset() {
        setMonsts([
            false, false, false, false, false, false, false, false, false, false, // P
            false, false, false, false, false, false, false, false, false, false, // A
            false, false, false, false, false, false, false, false, false, false, // X
            false, false // D, G
        ]);
        invoke("write", { v: 0b10000000 });
    }

    return (
        <>
            <div className='main'>
                <p className='title'>Monster Controller</p>
                <p className='subtitle'>v0.1.0 Frontend Vite React JS</p>
            </div>
            <div className='interface'>
                <table className='interface'>
                    <thead>
                        <tr className='interface h'>
                            {names.map((n, i) =>
                                <th key={i} className='interface' onDoubleClick={() => {
                                    let nn = prompt('Input the name of the monster');
                                    if (nn != null) {
                                        let nnames = names;
                                        nnames[i] = nn;
                                        setNames([...nnames]);
                                    }
                                }}>{n}{keyguide ? ` (${i + 1 < 10 ? i + 1 : 0})` : ''}</th>
                            )}
                        </tr>
                    </thead>
                    <tbody>
                        {[0, 1, 2].map((_, i1) =>
                            <tr key={i1} className='interface'>
                                {monsts.slice(0 + i1 * 10, 10 + i1 * 10).map((m, i) =>
                                    <td key={i} className={`i${enabled && inputtype == 1 ? '' : 'd'}${m ? 'a' : ''}`} onClick={() => {
                                        if (enabled && inputtype == 1) {
                                            monstparse(i + i1 * 10, !monsts[i + i1 * 10]);
                                        }
                                    }}>{i1 == 0 ? 'Power' : i1 == 1 ? 'Activate' : 'Alternate Activate'}{keyguide ? ` (${i1 == 0 ?
                                        (i == 0 ? 'q' : i == 1 ? 'w' : i == 2 ? 'e' : i == 3 ? 'r' : i == 4 ? 't' : i == 5 ? 'y' : i == 6 ? 'u' : i == 7 ? 'i' : i == 8 ? 'o' : 'p') :
                                        i1 == 1 ? (i == 0 ? 'a' : i == 1 ? 's' : i == 2 ? 'd' : i == 3 ? 'f' : i == 4 ? 'g' : i == 5 ? 'h' : i == 6 ? 'j' : i == 7 ? 'k' : i == 8 ? 'l' : ';') :
                                            (i == 0 ? 'z' : i == 1 ? 'x' : i == 2 ? 'c' : i == 3 ? 'v' : i == 4 ? 'b' : i == 5 ? 'n' : i == 6 ? 'm' : i == 7 ? ',' : i == 8 ? '.' : '/')
                                        })` : ''}</td>
                                )}
                            </tr>
                        )}
                        <tr className='interface'>
                            <td className={`i${keyguide ? 'a' : ''}`} colSpan={2} onClick={() => setGuide(!keyguide)}>Key Guide (=): {keyguide ? 'On' : 'Off'}</td>
                            <td className={`i${enabled ? '' : 'd'}`} colSpan={2} onClick={() => {
                                if (enabled) {
                                    let nv = !datamode;
                                    invoke("write", { v: nv ? 0b10000011 : 0b10000010 });
                                    setDatamode(nv);
                                }
                            }}>GUI Update{keyguide ? ' (-)' : ''}: {datamode ? "Wait for data" : "Instant"}</td>
                            <td className={`i${enabled ? '' : 'd'}`} colSpan={2} onClick={() => { if (enabled) { reset() } }}>R E S E T{keyguide ? ' (Enter)' : ''}</td>
                            <td className={`i${enabled && inputtype == 1 ? '' : 'd'}${monsts[30] ? 'a' : ''}`} colSpan={2} onClick={() => {
                                if (enabled && inputtype == 1) {
                                    monstparse(30, !monsts[30]);
                                }
                            }}>Door{keyguide ? ' (Space)' : ''}: {monsts[30] ? "Open" : "Closed"}</td>
                            <td className={`i${enabled && inputtype == 1 ? '' : 'd'}${monsts[31] ? 'a' : ''}`} colSpan={2} onMouseDown={() => {
                                if (enabled && inputtype == 1) {
                                    monstparse(31, true);
                                }
                            }} onMouseUp={() => {
                                if (enabled && inputtype == 1) {
                                    monstparse(31, false);
                                }
                            }}>Garbage{keyguide ? " (')" : ''}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div className='type'>
                <table className='type'>
                    <tbody className='t'>
                        <tr>
                            <td className={`t${inputtype == 0 ? 'a' : ''}`} onClick={() => setInput(0)}>{inputtype == 0 && keyguide ? '<- ' : ''}Keyboard{inputtype == 0 && keyguide ? ' ->' : ''}</td>
                            <td className={`t${inputtype == 1 ? 'a' : ''}`} onClick={() => setInput(1)}>{inputtype == 1 && keyguide ? '<- ' : ''}GUI+{inputtype == 1 && keyguide ? ' ->' : ''}</td>
                            <td className={`t${inputtype == 2 ? 'a' : ''}`} onClick={() => setInput(2)}>{inputtype == 2 && keyguide ? '<- ' : ''}Online{inputtype == 2 && keyguide ? ' ->' : ''}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </>
    )
}

export default App

// Garbage code (saved because I could need it later)
/*

<tr className='interface'>
    {monsts.slice(0, 10).map((m, i) =>
        <td key={i} className={`i${inputtype == 1 ? '' : 'd'}${m ? 'a' : ''}`} onClick={() => {
            if (inputtype == 1) {
                monstparse(i, !monsts[i]);
            }
        }}>Power{keyguide ? ` (${i == 0 ? 'q' : i == 1 ? 'w' : i == 2 ? 'e' : i == 3 ? 'r' : i == 4 ? 't' : i == 5 ? 'y' : i == 6 ? 'u' : i == 7 ? 'i' : i == 8 ? 'o' : 'p'})` : ''}</td>
    )}
</tr>
<tr className='interface'>
    {monsts.slice(10, 20).map((m, i) =>
        <td key={i} className={`i${inputtype == 1 ? '' : 'd'}${m ? 'a' : ''}`} onClick={() => {
            if (inputtype == 1) {
                monstparse(i + 10, !monsts[i + 10]);
            }
        }}>Activate{keyguide ? ` (${i == 0 ? 'a' : i == 1 ? 's' : i == 2 ? 'd' : i == 3 ? 'f' : i == 4 ? 'g' : i == 5 ? 'h' : i == 6 ? 'j' : i == 7 ? 'k' : i == 8 ? 'l' : ';'})` : ''}</td>
    )}
</tr>
<tr className='interface'>
    {monsts.slice(20, 30).map((m, i) =>
        <td key={i} className={`i${inputtype == 1 ? '' : 'd'}${m ? 'a' : ''}`} onClick={() => {
            if (inputtype == 1) {
                monstparse(i + 20, !monsts[i + 20]);
            }
        }}>Alternate Activate{keyguide ? ` (${i == 0 ? 'z' : i == 1 ? 'x' : i == 2 ? 'c' : i == 3 ? 'v' : i == 4 ? 'b' : i == 5 ? 'n' : i == 6 ? 'm' : i == 7 ? ',' : i == 8 ? '.' : '/'})` : ''}</td>
    )}
</tr>

*/
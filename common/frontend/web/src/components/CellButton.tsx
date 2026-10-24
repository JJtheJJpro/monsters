import { Button } from "@mui/joy";
import * as Consts from "./consts";

export default function CellButton(props: {
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
        fontFamily: "Doors",
        fontSize: "inherit",
        bgcolor: props.active ? Consts.MAIN_COLOR : Consts.BLACK,
        color: props.active ? Consts.BLACK : Consts.MAIN_COLOR,
        borderColor: Consts.MAIN_COLOR,
        "&:hover": {
            bgcolor: props.active ? Consts.HOVER_ACTIVE_COLOR : Consts.HOVER_COLOR,
        },
        "&:active": {
            bgcolor: props.active ? Consts.ACTIVE_ACTIVE_COLOR : Consts.ACTIVE_COLOR,
        },
        "&.Mui-disabled": {
            bgcolor: props.active ? Consts.DISABLED_COLOR : Consts.BLACK,
            color: props.active ? Consts.BLACK : Consts.DISABLED_COLOR,
            borderColor: props.active ? Consts.MAIN_COLOR : Consts.DISABLED_COLOR,
            cursor: "not-allowed",
            pointerEvents: "auto",
        },
        "&.Mui-disabled:hover": {
            bgcolor: props.active ? Consts.DISABLED_COLOR : Consts.BLACK,
            color: props.active ? Consts.BLACK : Consts.DISABLED_COLOR,
            borderColor: props.active ? Consts.MAIN_COLOR : Consts.DISABLED_COLOR,
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
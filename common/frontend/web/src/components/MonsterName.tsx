import { useEffect, useRef } from "react";
import { Button, DialogContent, DialogTitle, Input, Modal, ModalClose, ModalDialog } from "@mui/joy";
import CellButton from "./CellButton";

type MonsterNameProps = {
    name: string;
    renameOpen: boolean;
    renameDraft: string;
    onOpenRename: () => void;
    onRenameDraftChange: (value: string) => void;
    onRenameCommit: () => void;
    onRenameCancel: () => void;
};

export function MonsterName(props: MonsterNameProps) {
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

export default MonsterName;
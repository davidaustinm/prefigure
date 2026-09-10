import { ChangeEvent, forwardRef } from "react";

type HiddenFileInputProps = {
  accept?: string;
  multiple?: boolean;
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void;
};

const HiddenFileInput = forwardRef<HTMLInputElement, HiddenFileInputProps>(
  function HiddenFileInput({ accept, multiple = false, onChange }, ref) {
    return (
      <input
        ref={ref}
        type="file"
        accept={accept}
        multiple={multiple}
        hidden
        onChange={onChange}
      />
    );
  }
);

export default HiddenFileInput;

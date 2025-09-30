import { CopyButton } from "./CopyButton";

interface TechnicalTextProps {
  text: string;
  truncate?: boolean;
  copyable?: boolean;
  label?: string;
}

export const TechnicalText = ({ text, truncate = false, copyable = true, label }: TechnicalTextProps) => {
  const displayText = truncate && text.length > 16
    ? `${text.slice(0, 8)}...${text.slice(-6)}`
    : text;

  return (
    <div className="flex items-center gap-2">
      <code className="font-mono text-xs bg-muted px-2 py-1 rounded">
        {displayText}
      </code>
      {copyable && <CopyButton text={text} label={label} />}
    </div>
  );
};

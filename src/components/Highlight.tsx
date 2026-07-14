interface HighlightProps {
  text: string;
  terms: string[];
}

export function Highlight({ text, terms }: HighlightProps) {
  const usable = terms.map((term) => term.trim()).filter(Boolean);
  if (!usable.length) return <>{text}</>;
  const escaped = usable.map((term) => term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const expression = new RegExp(`(${escaped.join("|")})`, "gi");
  return (
    <>
      {text.split(expression).map((part, index) =>
        usable.some((term) => part.toLowerCase() === term.toLowerCase()) ? (
          <mark key={`${part}-${index}`}>{part}</mark>
        ) : (
          <span key={`${part}-${index}`}>{part}</span>
        ),
      )}
    </>
  );
}

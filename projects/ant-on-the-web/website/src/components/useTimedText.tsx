import { useState } from "react";

export type Setter<T> = T | (() => T);

export type TimedTextOptions = {
  ms?: number;
};

export function useTimedText(
  initial: Setter<string>,
  options?: TimedTextOptions
): [string, (t: Setter<string>) => void] {
  const [previousTimeout, setPreviousTimeout] = useState<
    NodeJS.Timeout | undefined
  >(undefined);
  const [text, setText] = useState(initial);

  const setTimedText = (t: Setter<string>) => {
    setText(t);

    if (t === "") {
      setPreviousTimeout(undefined);
      return;
    }

    setPreviousTimeout(
      setTimeout(() => {
        if (!previousTimeout) setText("");
      }, options?.ms ?? 3000)
    );
  };

  return [text, setTimedText];
}

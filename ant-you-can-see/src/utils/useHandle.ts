import type { newsletterSignup, suggestAnt } from "@/server/posts";
import React, { useState, FormEvent } from "react";

type PostAction = typeof suggestAnt | typeof newsletterSignup;
type HandleProps<A extends PostAction> = {
  messages: {
    valid: string;
    error: string;
  };
  inputName: string;
  constructInputData: (text: string) => Parameters<A>[number];
  clearInput: () => void;
  postAction: A;
  validator: (text: string) => { msg: string; valid: boolean };
};

export function useHandle<A extends PostAction>(props: HandleProps<A>) {
  const [handling, setHandling] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadingCounter, setLoadingCounter] = useState(0);
  const [validMsg, setValidMsg] = useState("");
  const [errorMsg, setErrorMsg] = useState("");

  function setMessage(setter: (x: any) => unknown, message: string): void {
    setter(message);
    createMsgTimeout(setter);
  }

  function createMsgTimeout(msgSetter: (x: any) => unknown) {
    setTimeout(() => msgSetter(""), 3000);
  }

  function startHandling() {
    const loadingInterval = setInterval(() => {
      setLoadingCounter((loadingCounter + 1) % 3);
    }, 100);

    setHandling(true);
    setLoading(true);
    props.clearInput();

    const handleTime = 3000;
    setTimeout(() => {
      setLoading(false);
      setHandling(false);
      clearInterval(loadingInterval);
    }, handleTime);
  }

  function handleError() {
    setValidMsg("");
    setMessage(setErrorMsg, props.messages.error);
  }

  async function handle(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (handling) {
      return;
    }
    const target = event.target as typeof event.target & {
      [x: string]: { value: string };
    };

    const value = target[props.inputName]?.value.trim();
    if (value === undefined) return setMessage(setErrorMsg, "input not found!");

    const { valid, msg } = props.validator(value);
    if (!valid) return setMessage(setErrorMsg, msg);

    startHandling();
    try {
      const input = props.constructInputData(value) as any;
      const res = await props.postAction(input);
      if (res.success) {
        setValidMsg(props.messages.valid);
        setErrorMsg("");
        createMsgTimeout(setValidMsg);
      } else {
        handleError();
      }
    } catch {
      handleError();
    }
  }

  return {
    validMsg,
    errorMsg,
    loadingMsg: loading ? "loading" + ".".repeat(loadingCounter) : "",
    handle,
  };
}

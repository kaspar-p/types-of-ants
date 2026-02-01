import { AntOnTheWebResponse, QueryResponse } from "@/server/rpc";
import { useState, FormEvent } from "react";

type HandleProps<Input> = {
  messages: {
    valid: string;
    error: string;
  };
  inputName: string;
  constructInputData: (text: string) => Input;
  clearInput: () => void;
  postAction: (
    inp: Input,
  ) => Promise<QueryResponse<AntOnTheWebResponse["__type"]>>;
  validator: (text: string) => { msg: string; valid: boolean };
};

export function useHandle<Input>(props: HandleProps<Input>) {
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
    const input = props.constructInputData(value) as any;
    const res = await props.postAction(input);
    switch (res.__status) {
      case 200: {
        setValidMsg(props.messages.valid);
        setErrorMsg("");
        createMsgTimeout(setValidMsg);
        break;
      }
      case 409: {
        setValidMsg("");
        setErrorMsg(res.msg.toLowerCase());
        createMsgTimeout(setErrorMsg);
        break;
      }
      case 400:
      case 500:
      default: {
        handleError();
        break;
      }
    }
  }

  return {
    validMsg,
    errorMsg,
    loadingMsg: loading ? "loading" + ".".repeat(loadingCounter) : "",
    handle,
  };
}

"use client";

import { newsletterSignup } from "@/server/posts";
import { useHandle } from "@/utils/useHandle";
import { useState } from "react";

function validator(text: string): { msg: string; valid: boolean } {
  let msg = "";
  if (
    !/(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])/.test(
      text
    )
  ) {
    msg = "invalid email!";
  }

  return {
    valid: msg === "",
    msg,
  };
}

export function NewsletterBox() {
  const [email, setEmail] = useState("");
  const { validMsg, loadingMsg, errorMsg, handle } = useHandle({
    postAction: newsletterSignup,
    clearInput: () => setEmail(""),
    constructInputData: (val: string) => ({ email: val }),
    inputName: "email",
    messages: {
      valid: "thanks, see you soon!",
      error: "error encountered, signup failed!",
    },
    validator,
  });

  const message = validMsg ? (
    <div className="text-green-600">{validMsg}</div>
  ) : errorMsg ? (
    <div className="text-red-600">{errorMsg}</div>
  ) : loadingMsg ? (
    <div className="text-blue-700">{loadingMsg}</div>
  ) : (
    ""
  );

  return (
    <div>
      <div className="pl-3">interested in monthly ant emails?</div>
      <form
        className="flex flex-row flex-wrap pl-2"
        autoComplete="off"
        onSubmit={(e) => handle(e)}
      >
        <input
          className="m-1"
          type="text"
          name="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
        <input className="m-1" type="submit" value="join monthly newsletter" />
      </form>
      <div className="ml-1 pl-3 h-2 mb-4">{message}</div>
    </div>
  );
}

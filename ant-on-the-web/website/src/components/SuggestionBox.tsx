"use client";

import React, { FormEvent, useState } from "react";
import { suggestAnt } from "../server/posts";
import { useHandle } from "@/utils/useActions_v2";

function validator(text: string): { valid: boolean; msg: string } {
  let msg = "";
  if (text.length <= 2) {
    msg = "ant too short!";
  } else if (text.length >= 100) {
    msg = "ant too long!";
  }

  return {
    valid: msg === "",
    msg,
  };
}

export function SuggestionBox() {
  const [ant, setAnt] = useState("");
  const { validMsg, loadingMsg, errorMsg, handle } = useHandle({
    postAction: suggestAnt,
    constructInputData: (val: string) => ({ suggestion_content: val }),
    clearInput: () => setAnt(""),
    inputName: "ant",
    messages: {
      valid: "thanks!",
      error: "error encountered, ant not processed!",
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
      <div className="pl-3">have any new ant suggestions?</div>
      <form
        className="flex flex-row flex-wrap pl-2"
        autoComplete="off"
        onSubmit={(event) => handle(event)}
      >
        <input
          className="m-1"
          type="text"
          name="ant"
          value={ant}
          onChange={(e) => setAnt(e.target.value)}
        />
        <input type="submit" className="m-1" value="submit ant suggestion" />
      </form>
      <div className="ml-1 pl-3 h-2 mb-4">{message}</div>
    </div>
  );
}

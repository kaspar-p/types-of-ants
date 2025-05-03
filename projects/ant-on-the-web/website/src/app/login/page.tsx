"use client";

import React, { useState } from "react";

function validateUsername(username: string): boolean {
  const r = /[a-z]{8}/g;
  const passes = r.test(username);
  return passes;
}

function validatePhoneNumber(phoneNumber: string): boolean {
  const r = /[0-9]{10}/g; // TODO: support more phone number formats
  return r.test(phoneNumber);
}

export default function LoginPage() {
  const [loginUnique, setLoginUnique] = useState("");
  const [signupUsername, setSignupUsername] = useState("");
  const [signupPhone, setSignupPhone] = useState("");
  const [signupEmail, setSignupEmail] = useState("");

  function handle(e: any) {
    e.preventDefault();
    console.log("Submitted: ", e.target.value, loginUnique);
  }

  return (
    <div className="h-full">
      <h2>login</h2>
      <form autoComplete="off" onSubmit={(event) => handle(event)}>
        <span>
          your username, phone number, or email:{" "}
          <input
            className="m-1"
            type="text"
            name="unique"
            placeholder="ex. kaspar"
            value={loginUnique}
            onChange={(e) => setLoginUnique(e.target.value)}
          />
        </span>
        <input type="submit" className="m-1" value="login (does nothing)" />
      </form>
      <h2>signup</h2>
      <div>
        <div className="mb-2">if you don&apos;t have an account, signup:</div>
        <form
          className="grid grid-cols-2 max-w-sm gap-0"
          autoComplete="off"
          onSubmit={(event) => handle(event)}
        >
          <span>your username: </span>
          <input
            className="m-1"
            type="text"
            name="unique"
            placeholder="ex. kaspar"
            value={signupUsername}
            onChange={(e) => setSignupUsername(e.target.value)}
          />
          <span>your phone number: </span>
          <input
            className="m-1"
            type="text"
            name="unique"
            placeholder="ex. 970-" // TODO: add a twilio account behind this and field texts as "typesofants.org"
            value={signupPhone}
            onChange={(e) => setSignupPhone(e.target.value)}
          />
          <span>your email: </span>
          <input
            className="m-1"
            type="text"
            name="unique"
            placeholder="ex. kaspar@typesofants.org"
            value={signupEmail}
            onChange={(e) => setSignupEmail(e.target.value)}
          />
          <input type="submit" className="m-1" value="signup (does nothing)" />
        </form>
      </div>
    </div>
  );
}

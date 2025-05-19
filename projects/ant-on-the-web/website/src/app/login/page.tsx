"use client";

import React from "react";
import { SignupBox } from "./signup";
import { LoginBox } from "./login";

export default function LoginPage() {
  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <div className="m-4">
        <h2>login</h2>
        <LoginBox />
      </div>
      <div className="m-4">
        <h2>signup</h2>
        <SignupBox />
      </div>
    </div>
  );
}

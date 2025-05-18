"use client";

import React from "react";
import { SignupBox } from "./signup";
import { LoginBox } from "./login";

export default function LoginPage() {
  return (
    <div className="h-full">
      <h2>login</h2>
      <LoginBox />

      <h2>signup</h2>
      <SignupBox />
    </div>
  );
}

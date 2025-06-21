"use client";

import React, { useContext } from "react";
import { SignupBox } from "./signup";
import { LoginBox } from "./login";
import { TwoFactorVerificationBox } from "./two-factor";
import { UserContext } from "@/state/userContext";

export default function LoginPage() {
  const { user, setUser } = useContext(UserContext);

  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>login</h2>
          <LoginBox />
          {user.weakAuth && (
            <>
              <h2>two-factor</h2>
              <TwoFactorVerificationBox />
            </>
          )}
        </div>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>signup</h2>
          <SignupBox />
          {user.weakAuth && (
            <>
              <h2>two-factor</h2>
              <TwoFactorVerificationBox />
            </>
          )}
        </div>
      </>
    </div>
  );
}

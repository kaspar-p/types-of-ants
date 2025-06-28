"use client";

import React, { useContext, useEffect } from "react";
import { SignupBox } from "./signup";
import { LoginBox } from "./login";
import { UserContext } from "@/state/userContext";
import { useRouter } from "next/navigation";

export default function LoginPage() {
  const { user } = useContext(UserContext);
  const { push } = useRouter();

  useEffect(() => {
    if (user.weakAuth && user.loggedIn) {
      push("/");
    }
  });

  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>login</h2>
          <LoginBox />
        </div>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>signup</h2>
          <SignupBox />
        </div>
      </>
    </div>
  );
}

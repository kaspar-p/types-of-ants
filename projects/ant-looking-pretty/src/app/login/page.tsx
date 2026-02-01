"use client";

import { useEffect, useState } from "react";
import { SignupBox } from "./signup";
import { LoginBox } from "./login";
import { useRouter } from "next/navigation";
import { useUser } from "@/app/UserProvider";

export default function LoginPage() {
  const { user } = useUser();
  const { push } = useRouter();

  const [weakAuth, setWeakAuth] = useState(false);

  useEffect(() => {
    if (user.loggedIn) {
      push("/");
    }
  });

  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>login</h2>
          <LoginBox setWeakAuth={setWeakAuth} />
        </div>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>signup</h2>
          <SignupBox setWeakAuth={setWeakAuth} />
        </div>
      </>
    </div>
  );
}

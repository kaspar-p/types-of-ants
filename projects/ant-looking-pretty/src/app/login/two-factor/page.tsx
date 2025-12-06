"use client";

import { useEffect } from "react";
import { TwoFactorVerificationBox } from "../two-factor";
import { useUser } from "@/state/userContext";
import { useRouter } from "next/navigation";

export default function TwoFactorPage() {
  const { user } = useUser();
  const { push } = useRouter();

  useEffect(() => {
    if (!user.weakAuth) {
      push("/login");
    }
  });

  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>two-factor</h2>
          <TwoFactorVerificationBox />
        </div>
      </>
    </div>
  );
}

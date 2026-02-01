"use client";

import { ChangePasswordsBox } from "@/components/ChangePasswordsBox";
import ChangeUsernameBox from "@/components/ChangeUsernameBox";
import { InputBanner } from "@/components/InputBanner";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import Link from "next/link";
import { useState } from "react";
import { useUser } from "../UserProvider";

const formatPhoneNumber = (p: string): string => {
  const r = /^\+(\d)(\d{3})(\d{3})(\d{4})$/;
  const matches = r.exec(p);
  if (!matches) return p;
  return `+${matches[1]} (${matches[2]}) ${matches[3]}-${matches[4]}`;
};

export default function ProfilePage() {
  const { user } = useUser();

  console.log(user);

  const [changingPassword, setChangingPassword] = useState<boolean>(false);
  const [changingUsername, setChangingUsername] = useState<boolean>(false);

  return (
    <ErrorBoundary isError={false}>
      <LoadingBoundary isLoading={false}>
        <div>
          <InputBanner />

          <div className="m-3">
            {!user.loggedIn ? (
              <h3>
                seems like you aren&apos;t logged in:{" "}
                <Link href={"/login"}>/login</Link>
              </h3>
            ) : (
              <div className="flex flex-col md:w-12 xl:w-3/12">
                <div>
                  <span className="min-w-min m-1">
                    username: {user.user.username}
                  </span>
                  <a
                    href="javascript:void"
                    className="min-w-min m-1 ml-1"
                    onClick={(e) => {
                      e.preventDefault();
                      setChangingUsername((s) => !s);
                    }}
                  >
                    change?
                  </a>
                </div>

                <span className="min-w-min m-1">
                  id: <code className="bg-slate-200">{user.user.userId}</code>
                </span>

                <span className="min-w-min m-1">
                  {user.user.emails.length > 0 ? (
                    <>
                      email{user.user.emails.length > 1 ? "s" : ""}:{" "}
                      <span>{user.user.emails.join(", ")}</span>
                    </>
                  ) : (
                    <>emails: none!</>
                  )}
                </span>

                <span className="min-w-min m-1">
                  {user.user.phoneNumbers.length > 0 ? (
                    <>
                      phone number
                      {user.user.phoneNumbers.length > 1 ? "s" : ""}:{" "}
                      <span>
                        {user.user.phoneNumbers
                          .map((p) => formatPhoneNumber(p))
                          .join(", ")}
                      </span>
                    </>
                  ) : (
                    <>phone numbers: none!</>
                  )}
                </span>

                <span className="min-w-min m-1">
                  created: <span>{user.user.joined.toLocaleString()}</span>
                </span>

                <button
                  className="min-w-min m-1"
                  onClick={() => setChangingPassword((s) => !s)}
                >
                  change password?
                </button>
                {changingPassword && (
                  <ChangePasswordsBox
                    secret={""}
                    onValid={() => {
                      setTimeout(() => setChangingPassword(false), 3000);
                    }}
                  />
                )}
                {changingUsername && (
                  <ChangeUsernameBox
                    onSuccess={async () => {
                      setTimeout(() => setChangingUsername(false), 3000);
                    }}
                  />
                )}
              </div>
            )}
          </div>
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}

"use client";

import { ChangePasswordsBox } from "@/components/ChangePasswordsBox";
import { NewsletterBox } from "@/components/NewsletterBox";
import { SuggestionBox } from "@/components/SuggestionBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { UserContext } from "@/state/userContext";
import Link from "next/link";
import { useContext, useState } from "react";

const formatPhoneNumber = (p: string): string => {
  const r = /^\+(\d)(\d{3})(\d{3})(\d{4})$/;
  const matches = r.exec(p);
  if (!matches) return p;
  return `+${matches[1]} (${matches[2]}) ${matches[3]}-${matches[4]}`;
};

export default function ProfilePage() {
  const { user } = useContext(UserContext);

  const [changingPassword, setChangingPassword] = useState<boolean>(false);

  return (
    <ErrorBoundary isError={false}>
      <LoadingBoundary isLoading={false}>
        <div>
          <div
            id="forms-container"
            style={{
              display: "flex",
              flexDirection: "row",
              flexWrap: "wrap",
              alignSelf: "center",
            }}
          >
            <SuggestionBox />
            <NewsletterBox />
          </div>

          <div className="m-3">
            {!(user.weakAuth && user.loggedIn) ? (
              <h3>
                seems like you aren&apos;t logged in:{" "}
                <Link href={"/login"}>/login</Link>
              </h3>
            ) : (
              <div className="flex flex-col md:w-12 xl:w-3/12">
                <span className="min-w-min m-1">
                  username: <span>{user.user.username}</span>
                </span>

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
                      phone number{user.user.phoneNumbers.length > 1 ? "s" : ""}
                      :{" "}
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
              </div>
            )}
          </div>
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}

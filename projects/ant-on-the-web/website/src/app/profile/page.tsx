"use client";

import { NewsletterBox } from "@/components/NewsletterBox";
import { SuggestionBox } from "@/components/SuggestionBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { UserContext } from "@/state/userContext";
import Link from "next/link";
import { useContext } from "react";

const formatPhoneNumber = (p: string): string => {
  const r = /^\+(\d)(\d{3})(\d{3})(\d{4})$/;
  const matches = r.exec(p);
  if (!matches) return p;
  return `+${matches[1]} (${matches[2]}) ${matches[3]}-${matches[4]}`;
};

export default function ProfilePage() {
  const { user } = useContext(UserContext);

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
            {user.weakAuth && user.loggedIn ? (
              <div className="flex flex-col">
                <span className="min-w-min">
                  username: <span>{user.user.username}</span>
                </span>

                <span className="min-w-min">
                  id: <span>{user.user.userId}</span>
                </span>

                <span className="min-w-min">
                  {user.user.emails.length > 0 ? (
                    <>
                      email{user.user.emails.length > 1 ? "s" : ""}:{" "}
                      <span>{user.user.emails.join(", ")}</span>
                    </>
                  ) : (
                    <>emails: none!</>
                  )}
                </span>

                <span className="min-w-min">
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

                <span className="min-w-min">
                  created: <span>{user.user.joined.toLocaleString()}</span>
                </span>
              </div>
            ) : (
              <h3>
                seems like you aren&apos;t logged in:{" "}
                <Link href={"/login"}>/login</Link>
              </h3>
            )}
          </div>
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}

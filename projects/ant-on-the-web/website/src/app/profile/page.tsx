"use client";

import { NewsletterBox } from "@/components/NewsletterBox";
import { SuggestionBox } from "@/components/SuggestionBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { getUser } from "@/server/queries";
import { UserContext } from "@/state/userContext";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { useContext } from "react";

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
            {user.loggedIn ? (
              <div className="flex flex-col">
                <span className="min-w-min">
                  username: <span>{user.user.username}</span>
                </span>

                <span className="min-w-min">
                  id: <span>{user.user.userId}</span>
                </span>

                <span className="min-w-min">
                  email{user.user.emails.length > 1 ? "s" : ""}:{" "}
                  <span>{user.user.emails.join(", ")}</span>
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

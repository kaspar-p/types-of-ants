"use client";

import React from "react";
import { AntBanner } from "../components/AntBanner";
import { escapeAnt } from "../utils/utils";
import { useQuery } from "../utils/useQuery";
import { getAllAnts, getReleaseNumber } from "../server/queries";
import { SuggestionBox } from "../components/SuggestionBox";
import { NewsletterBox } from "@/components/NewsletterBox";

export default function Home() {
  const {
    res: allAnts,
    loading: allAntsLoading,
    err: allAntsError,
  } = useQuery(getAllAnts);

  const {
    res: releaseNumber,
    loading: releaseNumberLoading,
    err: releaseNumberError,
  } = useQuery(getReleaseNumber);

  const error = allAntsError ?? releaseNumberError;
  if (error || !releaseNumber || !allAnts) {
    return (
      <div>
        Encountered error: {JSON.stringify(error)}
        <div>
          Error happened in{" "}
          {allAntsError
            ? "allAnts"
            : releaseNumberError
            ? "releaseNumber"
            : "none"}{" "}
          request
        </div>
      </div>
    );
  }

  const loading = allAntsLoading || releaseNumberLoading;
  if (loading) return <div>Sit tight...</div>;

  return (
    <div style={{ padding: "20px", fontFamily: "serif" }}>
      <h1>
        types of ants <span style={{ fontSize: "12pt" }}>v{releaseNumber}</span>
      </h1>
      <h2>ants discovered to date: {allAnts.ants.length}</h2>{" "}
      <h3>
        <a href="https://www.github.com/kaspar-p/types-of-ants">
          check out the code on github
        </a>
      </h3>
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
      <AntBanner />
      <div id="ant-filler">
        {allAnts.ants.map((ant, i) => (
          <div key={i}>{escapeAnt(ant)}</div>
        ))}
      </div>
    </div>
  );
}

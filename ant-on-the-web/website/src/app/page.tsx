"use client";

import React from "react";
import { AntBanner } from "../components/AntBanner";
import { escapeAnt } from "../utils/utils";
import { useActions } from "../utils/useActions";
import data from "../../data.json";
import { useQuery, Response } from "../utils/useQuery";
import { getAllAnts } from "../queries";

export default function Home() {
  const { actions, handle } = useActions();

  const {
    res: allAnts,
    loading: allAntsLoading,
    err: allAntsError,
  } = useQuery(getAllAnts);

  const versionNumber = 97;
  const antAmount = data.ants.length;
  const date = new Date().toLocaleDateString();

  return (
    <div style={{ padding: "20px", fontFamily: "serif" }}>
      <h1>
        types of ants <span style={{ fontSize: "12pt" }}>v{versionNumber}</span>
      </h1>
      <h2>ants discovered to date: {antAmount}</h2>{" "}
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
        <div id="new-ant-form-container">
          <div className="form-label">have any new ant suggestions?</div>
          <form
            className="form-form"
            id="new-ant-form"
            autoComplete="off"
            onSubmit={(e) => handle(e, actions.newAnt)}
          >
            <input className="form-text" id="new-ant" type="text" />
            <input
              type="submit"
              className="form-submit"
              value="submit ant suggestion"
            />
          </form>
          <div className="replacer" id="new-ant-replacer"></div>
        </div>
        <div id="newsletter-form-container">
          <div className="form-label">interested in monthly ant emails?</div>
          <form
            className="form-form"
            id="newsletter-form"
            autoComplete="off"
            onSubmit={(e) => handle(e, actions.newsletter)}
          >
            <input className="form-text" id="newsletter" type="text" />
            <input
              className="form-submit"
              type="submit"
              value="join monthly newsletter"
            />
          </form>
          <div className="replacer" id="newsletter-replacer"></div>
        </div>
      </div>
      <AntBanner />
      <div id="ant-filler">
        {allAntsLoading
          ? "Loading..."
          : allAntsError || !allAnts
          ? "ERROR"
          : allAnts.ants.map((ant, i) => <div key={i}>{escapeAnt(ant)}</div>)}
      </div>
    </div>
  );
}

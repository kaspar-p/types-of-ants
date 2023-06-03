"use client";
import React, { FormEvent } from "react";
import data from "../../data.json";

const useBanner = () => {
  const bannerId = "scroll-container";
  const helperWrapperId = "scroll-helper-wrapper";
  const pxPerSecond = 100;

  function setUpElements() {
    const banner = document.getElementById(bannerId);
    if (!banner) throw new Error("There was no element with id: " + bannerId);
    const currentHelperWrapper = document.getElementById(helperWrapperId);

    if (currentHelperWrapper) {
      const clones = currentHelperWrapper.querySelectorAll("[data-clone]");
      Array.prototype.forEach.call(clones, (clone) => clone.remove());

      const childrenCount = currentHelperWrapper.children.length;
      for (let i = 0; i < childrenCount; i++) {
        banner.appendChild(currentHelperWrapper.children[0]);
      }
      currentHelperWrapper.remove();
    }

    const bannerWidth = banner.getBoundingClientRect().width;
    const childWidths = Array.prototype.map.call(
      banner.children,
      (child) => child.getBoundingClientRect().width as any
    ) as number[];
    const widestChild = Math.max(...childWidths);
    const minWidthToCoverBanner = bannerWidth + widestChild;
    const childrenWidth = Array.prototype.reduce.call(
      banner.children,
      (total, child) => total + child.getBoundingClientRect().width,
      0
    ) as number;
    let currentWidth = childrenWidth;

    do {
      // Continue to add children, doubling, until width is reached
      Array.prototype.forEach.call(banner.children, (child) => {
        const clone = child.cloneNode();
        clone.setAttribute("aria-hidden", true);
        clone.dataset.clone = true;
        clone.innerText = child.innerText;
        banner.appendChild(clone);
      });
      currentWidth += childrenWidth;
    } while (currentWidth < minWidthToCoverBanner);

    const transitionWrapperHelper = document.createElement("div");
    transitionWrapperHelper.id = helperWrapperId;

    const childrenCount = banner.children.length;
    for (let i = 0; i < childrenCount; i++) {
      transitionWrapperHelper.appendChild(banner.children[0]);
    }
    banner.appendChild(transitionWrapperHelper);
    transitionWrapperHelper.dataset.childrenWidth = childrenWidth.toString();
  }

  function scrollBanner() {
    const banner = document.getElementById(bannerId);
    if (!banner) throw new Error("There was no element with id: " + bannerId);
    const helperWrapper = document.getElementById(helperWrapperId);
    if (!helperWrapper)
      throw new Error("There was no element with id: " + helperWrapperId);

    let childrenWidth: number;
    if (!helperWrapper.dataset.childrenWidth) {
      throw new Error("HelperWrapper had no dataset children width!");
    } else {
      childrenWidth = parseInt(helperWrapper.dataset.childrenWidth, 10);
    }
    const offsetLeft = helperWrapper.offsetLeft;

    if (offsetLeft <= Math.ceil(-1 * childrenWidth)) {
      helperWrapper.style.transitionDuration = "0s";
      helperWrapper.style.left = "0px";
      helperWrapper.style.removeProperty("transition-duration");
    } else if (
      helperWrapper.style.left === "" ||
      helperWrapper.style.left === "0px"
    ) {
      window.setTimeout(() => {
        helperWrapper.style.transitionDuration =
          (childrenWidth / pxPerSecond).toFixed() + "s";
        helperWrapper.style.left = -1 * childrenWidth + "px";
      }, 0);
    }

    window.requestAnimationFrame(scrollBanner);
  }

  window.addEventListener("load", () => {
    setUpElements();
    scrollBanner();
    window.addEventListener("resize", setUpElements);
  });
};

function useActions() {
  type Action = {
    containerID: string;
    replaceChildID: string;
    inputID: string;
    validator: (text: string) => Validation;
    endpoint: string;
    handling: keyof typeof actions;
  };

  type Validation = {
    valid: boolean;
    msg: string;
  };

  const develop = false;
  const isHandling = {
    newAnt: false,
    newsletter: false,
  };

  async function handle(event: FormEvent<HTMLFormElement>, action: Action) {
    event.preventDefault();

    const {
      containerID,
      replaceChildID,
      inputID,
      validator,
      endpoint,
      handling,
    } = action;

    const container = document.getElementById(containerID);
    if (!container) throw new Error("No element with id: " + containerID);
    const toReplace = document.getElementById(replaceChildID);
    if (!toReplace) throw new Error("No element with id: " + replaceChildID);

    const responseText = document.createElement("div");
    responseText.classList.add("replacer");

    container.replaceChild(responseText, toReplace);

    const input = document.getElementById(inputID) as HTMLInputElement;
    if (!input) throw new Error("No element with id: " + inputID);
    input.value = input.value.trim();
    if (isHandling[handling]) {
      return;
    }

    const { valid, msg } = validator(input.value);
    if (!valid) {
      responseText.style.color = "red";
      responseText.innerText = msg;
    } else {
      let counter = 0;
      const dotInterval = setInterval(() => {
        responseText.style.color = "blue";
        responseText.innerText = "loading" + ".".repeat(counter % 5);
        counter++;
      }, 100);

      // Send the request
      const url = develop
        ? "http://localhost:3000"
        : "https://www.kasparpoland.com";

      await fetch(`${url}/${endpoint}`, {
        method: "POST",
        body: input.value,
      })
        .then((response) => {
          clearInterval(dotInterval);
          if (response.ok && response.status === 200) {
          }
          return response.json();
        })
        .then((json) => {
          const { status, msg, userExists } = json;
          if (status === 200 && userExists) {
            responseText.style.color = "red";
            responseText.innerText = "you're already subscribed!";
          } else if (status === 200) {
            responseText.style.color = "green";
            responseText.innerText = "thanks!";
          } else {
            throw new Error(json);
          }
        })
        .catch((error) => {
          clearInterval(dotInterval);
          responseText.style.color = "red";
          responseText.innerText = "error encountered, input not processed!";
        });
    }

    // Make text appear and clear input
    isHandling[handling] = true;
    input.value = "";
    setTimeout(() => {
      container.replaceChild(toReplace, responseText);
      isHandling[handling] = false;
    }, 3000);
  }

  function newAntIsValid(text: string): Validation {
    let msg = "";
    if (text.length <= 2) {
      msg = "ant too short!";
    } else if (text.length >= 100) {
      msg = "ant too long!";
    }

    return {
      valid: msg === "",
      msg,
    };
  }

  function newsletterIsValid(text: string): Validation {
    let msg = "";
    if (
      !/(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])/.test(
        text
      )
    ) {
      msg = "invalid email!";
    }

    return {
      valid: msg === "",
      msg,
    };
  }

  const actions = {
    newAnt: {
      containerID: "new-ant-form-container",
      replaceChildID: "new-ant-replacer",
      inputID: "new-ant",
      validator: newAntIsValid,
      handling: "newAnt",
      endpoint: "api/new-ant",
    },
    newsletter: {
      containerID: "newsletter-form-container",
      replaceChildID: "newsletter-replacer",
      inputID: "newsletter",
      validator: newsletterIsValid,
      handling: "newsletter",
      endpoint: "api/ant-newsletter",
    },
  } as const;

  return { handle, actions };
}

export default function Home() {
  useBanner();
  const { actions, handle } = useActions();

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
      <div id="banner">
        <div>discovered 28 new ants on September 21, 2022:</div>
        <div id="scroll-container">
          {}
          <div className="banner-ant">silly ant</div>
          <div className="banner-ant">vampire ant (mosquito)</div>
          <div className="banner-ant">sad ant</div>
          <div className="banner-ant">prisoner ant</div>
          <div className="banner-ant">ant with daddy issues</div>
          <div className="banner-ant">ant on the ceiling (gonna fall down)</div>
          <div className="banner-ant">ant but dead</div>
          <div className="banner-ant">arsonist ant</div>
          <div className="banner-ant">tired ant</div>
          <div className="banner-ant">uncle ant</div>
          <div className="banner-ant">ant on a plane</div>
          <div className="banner-ant">ant selling wares</div>
          <div className="banner-ant">ant so long it looks weird</div>
          <div className="banner-ant">ant graduating from college</div>
          <div className="banner-ant">
            ant who made it to the top floor but found 0 crumbs and doesn't know
            how to get home
          </div>
          <div className="banner-ant">"ant"</div>
          <div className="banner-ant">official ant</div>
          <div className="banner-ant">lumpy ant</div>
          <div className="banner-ant">mac ant</div>
          <div className="banner-ant">caged ant</div>
          <div className="banner-ant">ant that has a bone to pick with you</div>
          <div className="banner-ant">
            ant that supplies ideas for this website
          </div>
          <div className="banner-ant">ant in a uhaul</div>
          <div className="banner-ant">they/them ant</div>
          <div className="banner-ant">windows ant</div>
          <div className="banner-ant">chill ant</div>
          <div className="banner-ant">ant on parole</div>
          <div className="banner-ant">ant at a funeral</div>
        </div>
      </div>
      <div id="ant-filler">
        {data.ants.map((antData) => (
          <div key={antData.ant}>{antData.ant}</div>
        ))}
      </div>
    </div>
  );
}

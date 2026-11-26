import type { ReactNode } from "react";
import { shortcutLabel, type ShortcutId } from "@/app/shortcuts";

/** A shortcut as it reads in a sentence, from the shortcuts table so it can't go stale. */
const K = ({ id }: { id: ShortcutId }) => <kbd className="font-mono text-xs">{shortcutLabel(id)}</kbd>;
/** A file, folder or value as typed. */
const C = ({ children }: { children: ReactNode }) => <code className="font-mono text-xs">{children}</code>;

export interface GuideSection {
  heading?: string;
  body: ReactNode[];
}

export interface GuidePage {
  id: string;
  title: string;
  sections: GuideSection[];
}

export const GUIDE_PAGES: GuidePage[] = [
  {
    id: "start",
    title: "Getting started",
    sections: [
      {
        body: [
          <>
            A project is a folder. cactus keeps everything in it as plain files, in the same layout Bruno uses, so you
            can commit it to git and open the same folder in Bruno.
          </>,
          <>
            Open a folder with <K id="openProject" />. An empty folder becomes a new project. The last project you had
            open comes back when you start cactus.
          </>,
          <>Change the look in Settings › Theme. Default dark and Default light are the main themes.</>,
        ],
      },
      {
        heading: "What's in the folder",
        body: [
          <>
            <C>opencollection.yml</C> holds defaults and variables shared by every request.
          </>,
          <>
            <C>environments/</C> has one file per environment.
          </>,
          <>Every other .yml file is a request. Folders group them.</>,
          <>
            <C>critters/</C> holds your critters and their flows.
          </>,
          <>
            <C>.cactus/</C> is for this machine only: secrets, history and captured values. It ignores itself in git.
          </>,
        ],
      },
      {
        heading: "Saving",
        body: [
          <>
            Saving changes only what you edited. Comments, formatting and anything cactus doesn't use stay as they were.
          </>,
        ],
      },
    ],
  },
  {
    id: "requests",
    title: "Requests",
    sections: [
      {
        body: [
          <>
            <K id="newScenario" /> makes a request. Pick the method, type the URL and press <K id="send" /> to send it.{" "}
            <K id="save" /> saves, <K id="duplicate" /> duplicates and <K id="switcher" /> jumps to any request by name.
          </>,
        ],
      },
      {
        heading: "Tabs",
        body: [
          <>Params: query and path parameters. Editing them updates the URL.</>,
          <>Headers and Body: what gets sent.</>,
          <>Auth: how the request signs in. Inherit uses the nearest folder's auth, or the project's.</>,
          <>Outputs: values to save from the response. See Variables.</>,
          <>Checks: tests on the response. See Responses.</>,
          <>Scripts: only shown when a file from Bruno or Postman has them. They're kept but not run.</>,
        ],
      },
      {
        heading: "Folders",
        body: [<>A folder can set headers, auth and variables for every request inside it.</>],
      },
      {
        heading: "Changes on disk",
        body: [
          <>
            If a file changes outside cactus while you have unsaved edits to it, a bar appears so you can reload it or
            keep yours.
          </>,
        ],
      },
    ],
  },
  {
    id: "variables",
    title: "Variables",
    sections: [
      {
        body: [
          <>
            Write <C>{"{{name}}"}</C> in a URL, header, body or auth field to use a variable. <K id="variables" /> opens
            the variables.
          </>,
        ],
      },
      {
        heading: "Environments",
        body: [
          <>
            An environment is a set of values, like staging or production. Pick one in the sidebar to switch between
            them.
          </>,
        ],
      },
      {
        heading: "Which value wins",
        body: [
          <>
            Values are looked up in this order, and later ones win: the project, the active environment, folders, the
            request, secrets, then captured values.
          </>,
        ],
      },
      {
        heading: "Capturing values",
        body: [
          <>
            In a request's Outputs tab, give a variable name and where to read it from, like <C>res.body.data.token</C>.
            After the request runs, later requests can use <C>{"{{token}}"}</C>.
          </>,
        ],
      },
    ],
  },
  {
    id: "secrets",
    title: "Secrets",
    sections: [
      {
        body: [
          <>
            Mark a variable as secret and only its name goes in the project. The value stays on this machine, in{" "}
            <C>.cactus/</C>. Everyone on the team sets their own.
          </>,
        ],
      },
    ],
  },
  {
    id: "responses",
    title: "Responses",
    sections: [
      {
        body: [
          <>
            The response shows next to the request. <K id="toggleResponse" /> hides or shows it.
          </>,
          <>Body shows the response, with search and a tree view for JSON. Headers lists what came back.</>,
          <>History keeps the last responses for each request.</>,
        ],
      },
      {
        heading: "Checks",
        body: [
          <>
            Add checks in the request's Checks tab, like <C>res.status eq 200</C>. After sending, the response's Checks
            tab shows which passed.
          </>,
        ],
      },
    ],
  },
  {
    id: "running",
    title: "Running and importing",
    sections: [
      {
        heading: "Run all",
        body: [
          <>
            Run all in the sidebar sends every request in order and lists what passed and failed. Click a result to open
            that request.
          </>,
        ],
      },
      {
        heading: "Import",
        body: [
          <>
            The import button in the sidebar takes a Postman collection or an OpenAPI file and adds its requests to the
            project. Variables it finds go in <C>opencollection.yml</C>.
          </>,
        ],
      },
    ],
  },
  {
    id: "critters",
    title: "Critters",
    sections: [
      {
        body: [
          <>
            A critter is a user you test as, like an admin or a new customer. Each one has its own variables, secrets
            and saved values. <K id="critters" /> shows them.
          </>,
        ],
      },
      {
        heading: "Making one",
        body: [
          <>
            Pick a species and a name. You can start empty, with the variable names from an environment, or as a copy of
            another critter.
          </>,
        ],
      },
      {
        heading: "Sending as a critter",
        body: [
          <>
            Pick a critter in the sidebar and every request is sent as them. Values a request captures are saved to that
            critter only. Its Context tab shows what it has saved, and when.
          </>,
        ],
      },
      {
        heading: "Logging in",
        body: [
          <>
            Give a critter a login flow in its Settings tab. When a request comes back 401, cactus runs the login flow
            and tries the request once more.
          </>,
        ],
      },
    ],
  },
  {
    id: "flows",
    title: "Flows",
    sections: [
      {
        body: [
          <>
            A flow is a list of steps a critter runs in order. Drag requests in from the list, and drag steps to reorder
            them. If a step fails, the rest are skipped.
          </>,
        ],
      },
      {
        heading: "Steps",
        body: [
          <>
            A step can change the status it expects, add checks and capture values. These apply only inside the flow.
            The request file isn't touched.
          </>,
          <>
            Tool steps do things other than send a request. Drag them in from the Input, Flow, Output and Security
            sections, under your requests. Values they set last for that run only.
          </>,
        ],
      },
      {
        heading: "Tools",
        body: [
          <>
            <b>Prompt</b> asks you for values when the flow runs.
          </>,
          <>
            <b>Pick from list</b> lets you choose one item from a list an earlier step captured, and sets variables from
            it.
          </>,
          <>
            <b>Display</b> shows a message. It can pause the run until you continue.
          </>,
          <>
            <b>Set values</b> sets variables for later steps. Use <C>$uuid</C>, <C>$timestamp</C>, <C>$isoTime</C>,{" "}
            <C>$randomInt</C> or <C>$randomEmail</C> to generate one.
          </>,
          <>
            <b>Assert</b> fails the flow unless values pass their checks, including every item of a list.
          </>,
          <>
            <b>Poll</b> sends a request again and again until its checks pass, like waiting for a job to finish.
          </>,
          <>
            <b>Wait</b> pauses for a while, like for an API that takes time to catch up.
          </>,
          <>
            <b>Run flow</b> runs another flow as one step, from this critter or another, so flows can share setup.
          </>,
          <>
            <b>Open in browser</b> opens a URL, like a sign-in page.
          </>,
          <>
            <b>Webhook</b> sends a request that isn't in the project, like telling another system the flow ran.
          </>,
          <>
            <b>Slack</b> posts a message to a Slack incoming webhook.
          </>,
          <>
            <b>Notify</b> shows a desktop notification.
          </>,
          <>
            <b>Fuzz</b> sends a request again with each value swapped for attack payloads, and reports what gives them
            away: SQL errors, stack traces, reflected markup, file contents.
          </>,
          <>
            <b>Certificate</b> checks a host&apos;s TLS certificate: dates, name, trust, key and signature.
          </>,
          <>
            <b>Rate limit</b> sends a request past its limit and checks the server starts answering 429.
          </>,
          <>
            <b>Load test</b> sends a request many times at once and measures response times.
          </>,
          <>
            <b>Security headers</b> checks a response&apos;s HSTS, CSP, framing, cookie flags and CORS, and whether it
            names the server&apos;s version.
          </>,
          <>
            Security tools fail on findings at or above <b>Fail on</b>, and <b>Set</b> hands their findings and metrics
            to later steps.
          </>,
        ],
      },
      {
        heading: "Editing",
        body: [
          <>
            Edit a flow visually or as YAML. Ctrl+Z and Ctrl+Shift+Z undo and redo. <K id="save" /> saves it. The Run
            tab runs it and shows each step.
          </>,
        ],
      },
    ],
  },
  {
    id: "schedules",
    title: "Schedules and alerts",
    sections: [
      {
        body: [
          <>
            Add a schedule to a flow to run it every so often, like every 5 minutes. Pick an environment for it, or let
            it use whichever is active.
          </>,
          <>
            Schedules only run while cactus is open. A flow never runs twice at once, and runs missed while the computer
            sleeps are skipped.
          </>,
        ],
      },
      {
        heading: "Alerts",
        body: [
          <>By default you're alerted when a flow starts failing and when it recovers.</>,
          <>You can also be alerted when a value it watches changes from the last run.</>,
          <>The last 50 runs of each flow are kept.</>,
        ],
      },
    ],
  },
  {
    id: "git",
    title: "Git",
    sections: [
      {
        body: [
          <>
            <K id="changes" /> shows what changed in the project. Stage, unstage, discard and commit from there, see
            each diff, and switch or create local branches.
          </>,
          <>
            Commits use your name and email from git, or from Settings › Git. Git hooks don't run. Push and pull from
            your usual git tool.
          </>,
        ],
      },
    ],
  },
  {
    id: "mcp",
    title: "MCP",
    sections: [
      {
        body: [
          <>
            cactus can run an MCP server so AI agents can use the open project. Agents send requests and run flows
            through cactus, so they never see your secrets or tokens.
          </>,
          <>
            Turn it on in Settings › MCP. It only listens on this machine, and every call needs the token shown there.
          </>,
        ],
      },
      {
        heading: "What agents can do",
        body: [
          <>List and read requests, environments, critters, flows, variables and history. Secret values stay hidden.</>,
          <>Send requests and run folders and flows. These show up in history like your own.</>,
          <>With Allow edits on, create and save requests and flows.</>,
        ],
      },
      {
        heading: "Connecting",
        body: [
          <>Copy config gives you the JSON most agents take. Copy Claude Code command gives you a command to paste.</>,
          <>New token replaces the old one, so agents using it need the new one.</>,
        ],
      },
    ],
  },
  {
    id: "cli",
    title: "Command line",
    sections: [
      {
        body: [<>cactus-cli runs a project without the app, for example in CI.</>],
      },
      {
        heading: "Commands",
        body: [
          <>
            <C>cactus-cli run &lt;project&gt; [folder]</C> sends every request, or every request in one folder.
          </>,
          <>
            <C>cactus-cli send &lt;project&gt; &lt;file&gt;</C> sends one request and prints the body.
          </>,
          <>
            <C>cactus-cli flow &lt;project&gt; &lt;critter&gt; &lt;flow&gt;</C> runs a flow. <C>--all</C> runs every
            flow.
          </>,
          <>
            <C>cactus-cli import &lt;project&gt; &lt;file&gt;</C> imports Postman or OpenAPI.
          </>,
        ],
      },
      {
        heading: "Options",
        body: [
          <>
            <C>--env NAME</C> picks the environment.
          </>,
          <>
            For run and send, <C>--as CRITTER</C> sends as a critter. For run and flow, <C>--json</C> prints JSON
            instead of a report.
          </>,
          <>
            For flows, <C>--junit FILE</C> writes a JUnit report and <C>--var name=value</C> answers a prompt step.
          </>,
        ],
      },
    ],
  },
];

// some default named functions are tb/links, tb/togglebot
type named_function = `@${string}/${string}`;
type raw_string = string | `$${string}`;
type arg_type = "url" | "string" | `string${number}` | `string...`;
type parse_mode = "?" | "!";
type argument =
  | `${arg_type}`
  | `${arg_type}${parse_mode}${string}`
  // Syntax: type ?|! parser_argument <!> error_message
  | `${arg_type}${parse_mode}${string}<!>${string}`;

// One of the platforms specified in config.platforms
type platform = string;

type config = {
  // path to config files to included
  // this is only read in the main-config
  // all configs will be loaded in the order specified (main-config last), 
  // later values override those already set by earlier files
  // Validation happens after reading all files 
  // (child-config can use things specified in main-config)
  include?: string[];
  // platforms?:
  // | {
  //   twitch?: {
  //     login: string;
  //     channel: string;
  //     token: string;
  //   };
  //   discord?: {
  //     token: string;
  //   };
  // }
  // | {
  //   [key: string]:
  //   | {
  //     type: "twitch";
  //     login: string;
  //     channel: string;
  //     token: string;
  //   }
  //   | {
  //     type: "discord";
  //     token: string;
  //   };
  // };
  //These names will automatically be prefixed with ! so just "lark"
  commands?: {
    [key: string]:
    | raw_string
    //:w| named_function
    | {
      args?: argument | argument[];
      action:
      | string
      | named_function
      | { [key: platform]: string | named_function };
      cooldown?: number | { [key: platform]: number };
      aliases?: string | string[];
      platforms?: platform | platform[];
    };
  };
  matches?: (
    | {
      //these will **not** automatically be prefixed with ! so "!lark"
      names: string | string[];
      platforms?: platform | platform[];
      args?: argument | argument[];
      action:
      | string
      | named_function
      | { [key: platform]: string | named_function };
      cooldown?: number;
    }
    | {
      regex: string;
      platforms?: platform | platform[];
      args?: argument | argument[];
      action:
      | string
      | named_function
      | { [key: platform]: string | named_function };
      cooldown?: number;
    }
  )[];
  // these functions will be named "local/${key}"
  functions?: {
    [key: string]: string;
  };
  constants?: {
    [key: string | named_function]:
    | { [key: string]: string }
    | { [key: string]: number }
    | {
      [key: string]:
      | { [key: string]: string }
      | { [key: string]: number };
    };
  };
};

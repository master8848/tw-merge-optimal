export type ClassValue = string | null | undefined | false | ClassValue[];

/** A type sugar validator tag for a config spec: callable in JS, the merge
 *  machinery reads its `.t` type code (one-to-one with the engine's TYPES). */
export type Validator = ((value: string) => boolean) & { readonly t: number };

/** One class group spec: a bare class, a list of classes, or prefix-keyed
 *  lists of keywords / `--theme-*` keys / `<type>` sugar strings / validators
 *  (the `''` prefix is the bare class itself). */
export type ClassGroup =
    | string
    | string[]
    | { [prefix: string]: (string | Validator)[] };

/** tailwind-merge-style plugin config. */
export interface PluginConfig {
    classGroups?: Record<string, ClassGroup[]>;
    conflictingClassGroups?: Record<string, string[]>;
    extend?: {
        classGroups?: Record<string, ClassGroup[]>;
        conflictingClassGroups?: Record<string, string[]>;
    };
}

export type Config = PluginConfig;

/** Config transformer: receives the previous config, returns the next one. */
export type MergeConfigFn = (prevConfig: PluginConfig) => PluginConfig;

/** A twMerge instance bound to a specific config. */
export type TwMerge = (...classes: (string | null | undefined | false)[]) => string;

/** Merge class strings: variadic, any mix of strings and falsy values. */
export declare function twMerge(...classes: ClassValue[]): string;

/** clsx-style join: strings and nested arrays, falsy values skipped. */
export declare function twJoin(...classes: ClassValue[]): string;

/** Set the result-cache bound (both whole-call and parse caches). `0` disables caching. Default: 8192. */
export declare function setCacheSize(cacheSize: number): void;

/** Build a new `twMerge` bound to the given config (or a transformer over the
 *  previous one). Returns a fresh function; the default export is unaffected. */
export declare function extendTailwindMerge(config: PluginConfig | MergeConfigFn): TwMerge;

/** Alias of `extendTailwindMerge`. */
export declare function createTailwindMerge(config: PluginConfig | MergeConfigFn): TwMerge;

/** Merge two plugin configs: classGroups append (group items concatenated,
 *  top-level and `extend`-wrapped treated identically — no replacement),
 *  conflictingClassGroups union. Returns a new object; inputs are untouched. */
export declare function mergeConfigs(a: PluginConfig, b: PluginConfig): PluginConfig;

export declare const validators: {
    isAny: Validator;
    isNumber: Validator;
    isInteger: Validator;
    isPercentage: Validator;
    isFraction: Validator;
    isTshirtSize: Validator;
    isLength: Validator;
    isShadow: Validator;
    isImage: Validator;
    isUrl: Validator;
    isPosition: Validator;
    isRatio: Validator;
    isWeight: Validator;
    isFamilyName: Validator;
    isAngle: Validator;
    isTime: Validator;
    isCustomIdent: Validator;
    isSpacing: Validator;
    isAnyNonArbitrary: Validator;
    isArbitraryLength: Validator;
    isArbitraryNumber: Validator;
    isArbitraryInteger: Validator;
    isArbitraryPercent: Validator;
    isArbitraryFraction: Validator;
    isArbitrarySize: Validator;
    isArbitraryPosition: Validator;
    isArbitraryShadow: Validator;
    isArbitraryImage: Validator;
    isArbitraryWeight: Validator;
    isArbitraryFamilyName: Validator;
    isArbitraryAngle: Validator;
    isArbitraryTime: Validator;
    isArbitraryRatio: Validator;
    isArbitraryIdent: Validator;
    isArbitraryUrl: Validator;
    isArbitraryString: Validator;
    isArbitraryCustomProperty: Validator;
    isArbitraryFunction: Validator;
    isArbitraryAny: Validator;
    isVariableLength: Validator;
    isVariableNumber: Validator;
    isVariableInteger: Validator;
    isVariablePercent: Validator;
    isVariableFraction: Validator;
    isVariableSize: Validator;
    isVariablePosition: Validator;
    isVariableShadow: Validator;
    isVariableImage: Validator;
    isVariableWeight: Validator;
    isVariableFamilyName: Validator;
    isVariableAngle: Validator;
    isVariableTime: Validator;
    isVariableIdent: Validator;
    isVariableUrl: Validator;
    isVariableString: Validator;
    isVariableCustomProperty: Validator;
    isVariableAny: Validator;
};

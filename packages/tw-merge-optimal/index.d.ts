export type ClassValue = string | null | undefined | false | ClassValue[];

/** Merge class strings. Accepts exactly one string — the shape `clsx()`-based
 *  `cn()` utils actually produce (`twMerge(clsx(...))`), so the hot path skips
 *  all rest-arg/array handling. Pass multiple arguments to `twMergeJoin`
 *  instead. */
export declare function twMerge(classString: string): string;

/** tailwind-merge-compatible `twMerge`: variadic, any mix of strings, falsy
 *  values and nested arrays. Same merge semantics as `twMerge`. */
export declare function twMergeJoin(...classes: ClassValue[]): string;

/** clsx-style join: strings and nested arrays, falsy values skipped. */
export declare function twJoin(...classes: ClassValue[]): string;

/** Set the result-cache bound (both whole-call and parse caches). `0` disables caching. Default: 8192. */
export declare function setCacheSize(cacheSize: number): void;

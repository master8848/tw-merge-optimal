export type ClassValue = string | null | undefined | false | ClassValue[];

export declare function twMerge(...classes: ClassValue[]): string;
export declare function twJoin(...classes: ClassValue[]): string;

/** Set the result-cache bound (both whole-call and parse caches). `0` disables caching. Default: 8192. */
export declare function setCacheSize(cacheSize: number): void;

export type ClassValue = string | null | undefined | false | ClassValue[];

export declare function twMerge(...classes: ClassValue[]): string;
export declare function twJoin(...classes: ClassValue[]): string;

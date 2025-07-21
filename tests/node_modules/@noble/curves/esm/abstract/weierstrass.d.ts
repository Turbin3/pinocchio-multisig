import { type CHash, type Hex, type PrivKey } from '../utils.ts';
import { type AffinePoint, type BasicCurve, type CurveInfo, type CurvePoint, type CurvePointCons } from './curve.ts';
import { type IField, type NLength } from './modular.ts';
export type { AffinePoint };
export type HmacFnSync = (key: Uint8Array, ...messages: Uint8Array[]) => Uint8Array;
type EndoBasis = [[bigint, bigint], [bigint, bigint]];
/**
 * When Weierstrass curve has `a=0`, it becomes Koblitz curve.
 * Koblitz curves allow using **efficiently-computable GLV endomorphism ψ**.
 * Endomorphism uses 2x less RAM, speeds up precomputation by 2x and ECDH / key recovery by 20%.
 * For precomputed wNAF it trades off 1/2 init time & 1/3 ram for 20% perf hit.
 *
 * Endomorphism consists of beta, lambda and splitScalar:
 *
 * 1. GLV endomorphism ψ transforms a point: `P = (x, y) ↦ ψ(P) = (β·x mod p, y)`
 * 2. GLV scalar decomposition transforms a scalar: `k ≡ k₁ + k₂·λ (mod n)`
 * 3. Then these are combined: `k·P = k₁·P + k₂·ψ(P)`
 * 4. Two 128-bit point-by-scalar multiplications + one point addition is faster than
 *    one 256-bit multiplication.
 *
 * where
 * * beta: β ∈ Fₚ with β³ = 1, β ≠ 1
 * * lambda: λ ∈ Fₙ with λ³ = 1, λ ≠ 1
 * * splitScalar decomposes k ↦ k₁, k₂, by using reduced basis vectors.
 *   Gauss lattice reduction calculates them from initial basis vectors `(n, 0), (-λ, 0)`
 *
 * Check out `test/misc/endomorphism.js` and
 * [gist](https://gist.github.com/paulmillr/eb670806793e84df628a7c434a873066).
 */
export type EndomorphismOpts = {
    beta: bigint;
    basises?: EndoBasis;
    splitScalar?: (k: bigint) => {
        k1neg: boolean;
        k1: bigint;
        k2neg: boolean;
        k2: bigint;
    };
};
export type BasicWCurve<T> = BasicCurve<T> & {
    a: T;
    b: T;
    allowedPrivateKeyLengths?: readonly number[];
    wrapPrivateKey?: boolean;
    endo?: EndomorphismOpts;
    isTorsionFree?: (c: WeierstrassPointCons<T>, point: WeierstrassPoint<T>) => boolean;
    clearCofactor?: (c: WeierstrassPointCons<T>, point: WeierstrassPoint<T>) => WeierstrassPoint<T>;
};
export type ScalarEndoParts = {
    k1neg: boolean;
    k1: bigint;
    k2neg: boolean;
    k2: bigint;
};
/**
 * Splits scalar for GLV endomorphism.
 */
export declare function _splitEndoScalar(k: bigint, basis: EndoBasis, n: bigint): ScalarEndoParts;
export type ECDSASigFormat = 'compact' | 'der';
export type Entropy = Hex | boolean;
export type SignOpts = Partial<{
    lowS: boolean;
    extraEntropy: Entropy;
    prehash: boolean;
    format: ECDSASigFormat | 'js';
}>;
export type VerOpts = Partial<{
    lowS: boolean;
    prehash: boolean;
    format: ECDSASigFormat | 'js' | undefined;
}>;
/** Instance methods for 3D XYZ projective points. */
export interface WeierstrassPoint<T> extends CurvePoint<T, WeierstrassPoint<T>> {
    /** projective X coordinate. Different from affine x. */
    readonly X: T;
    /** projective Y coordinate. Different from affine y. */
    readonly Y: T;
    /** projective z coordinate */
    readonly Z: T;
    /** affine x coordinate. Different from projective X. */
    get x(): T;
    /** affine y coordinate. Different from projective Y. */
    get y(): T;
    /** Encodes point using IEEE P1363 (DER) encoding. First byte is 2/3/4. Default = isCompressed. */
    toBytes(isCompressed?: boolean): Uint8Array;
    toHex(isCompressed?: boolean): string;
    /** @deprecated use .X */
    readonly px: T;
    /** @deprecated use .Y */
    readonly py: T;
    /** @deprecated use .Z */
    readonly pz: T;
    /** @deprecated use `toBytes` */
    toRawBytes(isCompressed?: boolean): Uint8Array;
    /** @deprecated use `multiplyUnsafe` */
    multiplyAndAddUnsafe(Q: WeierstrassPoint<T>, a: bigint, b: bigint): WeierstrassPoint<T> | undefined;
    /** @deprecated use `p.y % 2n === 0n` */
    hasEvenY(): boolean;
    /** @deprecated use `p.precompute(windowSize)` */
    _setWindowSize(windowSize: number): void;
}
/** Static methods for 3D XYZ projective points. */
export interface WeierstrassPointCons<T> extends CurvePointCons<T, WeierstrassPoint<T>> {
    /** Does NOT validate if the point is valid. Use `.assertValidity()`. */
    new (X: T, Y: T, Z: T): WeierstrassPoint<T>;
    /** @deprecated use `Point.BASE.multiply(Point.Fn.fromBytes(privateKey))` */
    fromPrivateKey(privateKey: PrivKey): WeierstrassPoint<T>;
    /** @deprecated use `import { normalizeZ } from '@noble/curves/abstract/curve.js';` */
    normalizeZ(points: WeierstrassPoint<T>[]): WeierstrassPoint<T>[];
    /** @deprecated use `import { pippenger } from '@noble/curves/abstract/curve.js';` */
    msm(points: WeierstrassPoint<T>[], scalars: bigint[]): WeierstrassPoint<T>;
}
/** @deprecated use WeierstrassPoint */
export type ProjPointType<T> = WeierstrassPoint<T>;
/** @deprecated use WeierstrassPointCons */
export type ProjConstructor<T> = WeierstrassPointCons<T>;
export type CurvePointsType<T> = BasicWCurve<T> & {
    fromBytes?: (bytes: Uint8Array) => AffinePoint<T>;
    toBytes?: (c: WeierstrassPointCons<T>, point: WeierstrassPoint<T>, isCompressed: boolean) => Uint8Array;
};
export type CurvePointsTypeWithLength<T> = Readonly<CurvePointsType<T> & Partial<NLength>>;
export type CurvePointsRes<T> = {
    Point: WeierstrassPointCons<T>;
    /** @deprecated the property will be removed in next release */
    CURVE: CurvePointsType<T>;
    /** @deprecated use `Point` */
    ProjectivePoint: WeierstrassPointCons<T>;
    /** @deprecated use `Point.Fn.fromBytes(privateKey)` */
    normPrivateKeyToScalar: (key: PrivKey) => bigint;
    /** @deprecated */
    weierstrassEquation: (x: T) => T;
    /** @deprecated use `Point.Fn.isValidNot0(num)` */
    isWithinCurveOrder: (num: bigint) => boolean;
};
/**
 * Weierstrass curve options.
 *
 * * p: prime characteristic (order) of finite field, in which arithmetics is done
 * * n: order of prime subgroup a.k.a total amount of valid curve points
 * * h: cofactor, usually 1. h*n is group order; n is subgroup order
 * * a: formula param, must be in field of p
 * * b: formula param, must be in field of p
 * * Gx: x coordinate of generator point a.k.a. base point
 * * Gy: y coordinate of generator point
 */
export type WeierstrassOpts<T> = Readonly<{
    p: bigint;
    n: bigint;
    h: bigint;
    a: T;
    b: T;
    Gx: T;
    Gy: T;
}>;
export type WeierstrassExtraOpts<T> = Partial<{
    Fp: IField<T>;
    Fn: IField<bigint>;
    allowInfinityPoint: boolean;
    endo: EndomorphismOpts;
    isTorsionFree: (c: WeierstrassPointCons<T>, point: WeierstrassPoint<T>) => boolean;
    clearCofactor: (c: WeierstrassPointCons<T>, point: WeierstrassPoint<T>) => WeierstrassPoint<T>;
    fromBytes: (bytes: Uint8Array) => AffinePoint<T>;
    toBytes: (c: WeierstrassPointCons<T>, point: WeierstrassPoint<T>, isCompressed: boolean) => Uint8Array;
}>;
/**
 * Options for ECDSA signatures over a Weierstrass curve.
 */
export type ECDSAOpts = Partial<{
    lowS: boolean;
    hmac: HmacFnSync;
    randomBytes: (bytesLength?: number) => Uint8Array;
    bits2int: (bytes: Uint8Array) => bigint;
    bits2int_modN: (bytes: Uint8Array) => bigint;
}>;
/** ECDSA is only supported for prime fields, not Fp2 (extension fields). */
export interface ECDSA {
    keygen: (seed?: Uint8Array) => {
        secretKey: Uint8Array;
        publicKey: Uint8Array;
    };
    getPublicKey: (secretKey: PrivKey, isCompressed?: boolean) => Uint8Array;
    sign: (msgHash: Hex, secretKey: PrivKey, opts?: SignOpts) => ECDSASigRecovered;
    verify: (signature: Hex | SignatureLike, msgHash: Hex, publicKey: Hex, opts?: VerOpts) => boolean;
    getSharedSecret: (secretKeyA: PrivKey, publicKeyB: Hex, isCompressed?: boolean) => Uint8Array;
    Point: WeierstrassPointCons<bigint>;
    Signature: ECDSASignatureCons;
    utils: {
        isValidSecretKey: (secretKey: PrivKey) => boolean;
        isValidPublicKey: (publicKey: Uint8Array, isCompressed?: boolean) => boolean;
        randomSecretKey: (seed?: Uint8Array) => Uint8Array;
        /** @deprecated use `randomSecretKey` */
        randomPrivateKey: (seed?: Uint8Array) => Uint8Array;
        /** @deprecated use `isValidSecretKey` */
        isValidPrivateKey: (secretKey: PrivKey) => boolean;
        /** @deprecated use `Point.Fn.fromBytes()` */
        normPrivateKeyToScalar: (key: PrivKey) => bigint;
        /** @deprecated use `point.precompute()` */
        precompute: (windowSize?: number, point?: WeierstrassPoint<bigint>) => WeierstrassPoint<bigint>;
    };
    info: CurveInfo;
}
export declare class DERErr extends Error {
    constructor(m?: string);
}
export type IDER = {
    Err: typeof DERErr;
    _tlv: {
        encode: (tag: number, data: string) => string;
        decode(tag: number, data: Uint8Array): {
            v: Uint8Array;
            l: Uint8Array;
        };
    };
    _int: {
        encode(num: bigint): string;
        decode(data: Uint8Array): bigint;
    };
    toSig(hex: string | Uint8Array): {
        r: bigint;
        s: bigint;
    };
    hexFromSig(sig: {
        r: bigint;
        s: bigint;
    }): string;
};
/**
 * ASN.1 DER encoding utilities. ASN is very complex & fragile. Format:
 *
 *     [0x30 (SEQUENCE), bytelength, 0x02 (INTEGER), intLength, R, 0x02 (INTEGER), intLength, S]
 *
 * Docs: https://letsencrypt.org/docs/a-warm-welcome-to-asn1-and-der/, https://luca.ntop.org/Teaching/Appunti/asn1.html
 */
export declare const DER: IDER;
export declare function _legacyHelperEquat<T>(Fp: IField<T>, a: T, b: T): (x: T) => T;
export declare function _normFnElement(Fn: IField<bigint>, key: PrivKey): bigint;
export declare function weierstrassN<T>(CURVE: WeierstrassOpts<T>, curveOpts?: WeierstrassExtraOpts<T>): WeierstrassPointCons<T>;
/** @deprecated use `weierstrass` in newer releases */
export declare function weierstrassPoints<T>(c: CurvePointsTypeWithLength<T>): CurvePointsRes<T>;
export interface ECDSASignature {
    readonly r: bigint;
    readonly s: bigint;
    readonly recovery?: number;
    addRecoveryBit(recovery: number): ECDSASigRecovered;
    hasHighS(): boolean;
    normalizeS(): ECDSASignature;
    recoverPublicKey(msgHash: Hex): WeierstrassPoint<bigint>;
    toBytes(format?: string): Uint8Array;
    toHex(format?: string): string;
    /** @deprecated */
    assertValidity(): void;
    /** @deprecated use `.toBytes('compact')` */
    toCompactRawBytes(): Uint8Array;
    /** @deprecated use `.toBytes('compact')` */
    toCompactHex(): string;
    /** @deprecated use `.toBytes('der')` */
    toDERRawBytes(): Uint8Array;
    /** @deprecated use `.toBytes('der')` */
    toDERHex(): string;
}
export type SignatureType = ECDSASignature;
export type ECDSASigRecovered = ECDSASignature & {
    readonly recovery: number;
};
export type RecoveredSignatureType = ECDSASigRecovered;
export type ECDSASignatureCons = {
    new (r: bigint, s: bigint, recovery?: number): ECDSASignature;
    fromBytes(bytes: Uint8Array, format?: ECDSASigFormat): ECDSASignature;
    fromHex(hex: string, format?: ECDSASigFormat): ECDSASignature;
    /** @deprecated use `.fromBytes(bytes, 'compact')` */
    fromCompact(hex: Hex): ECDSASignature;
    /** @deprecated use `.fromBytes(bytes, 'der')` */
    fromDER(hex: Hex): ECDSASignature;
};
export type SignatureLike = {
    r: bigint;
    s: bigint;
};
export type PubKey = Hex | WeierstrassPoint<bigint>;
export type CurveType = BasicWCurve<bigint> & {
    hash: CHash;
    hmac?: HmacFnSync;
    randomBytes?: (bytesLength?: number) => Uint8Array;
    lowS?: boolean;
    bits2int?: (bytes: Uint8Array) => bigint;
    bits2int_modN?: (bytes: Uint8Array) => bigint;
};
export type CurveFn = {
    /** @deprecated the property will be removed in next release */
    CURVE: CurvePointsType<bigint>;
    keygen: ECDSA['keygen'];
    getPublicKey: ECDSA['getPublicKey'];
    getSharedSecret: ECDSA['getSharedSecret'];
    sign: ECDSA['sign'];
    verify: ECDSA['verify'];
    Point: WeierstrassPointCons<bigint>;
    /** @deprecated use `Point` */
    ProjectivePoint: WeierstrassPointCons<bigint>;
    Signature: ECDSASignatureCons;
    utils: ECDSA['utils'];
    info: CurveInfo;
};
/**
 * Implementation of the Shallue and van de Woestijne method for any weierstrass curve.
 * TODO: check if there is a way to merge this with uvRatio in Edwards; move to modular.
 * b = True and y = sqrt(u / v) if (u / v) is square in F, and
 * b = False and y = sqrt(Z * (u / v)) otherwise.
 * @param Fp
 * @param Z
 * @returns
 */
export declare function SWUFpSqrtRatio<T>(Fp: IField<T>, Z: T): (u: T, v: T) => {
    isValid: boolean;
    value: T;
};
/**
 * Simplified Shallue-van de Woestijne-Ulas Method
 * https://www.rfc-editor.org/rfc/rfc9380#section-6.6.2
 */
export declare function mapToCurveSimpleSWU<T>(Fp: IField<T>, opts: {
    A: T;
    B: T;
    Z: T;
}): (u: T) => {
    x: T;
    y: T;
};
/**
 * Creates ECDSA for given elliptic curve Point and hash function.
 */
export declare function ecdsa(Point: WeierstrassPointCons<bigint>, hash: CHash, ecdsaOpts?: ECDSAOpts): ECDSA;
export type WsPointComposed<T> = {
    CURVE: WeierstrassOpts<T>;
    curveOpts: WeierstrassExtraOpts<T>;
};
export type WsComposed = {
    CURVE: WeierstrassOpts<bigint>;
    hash: CHash;
    curveOpts: WeierstrassExtraOpts<bigint>;
    ecdsaOpts: ECDSAOpts;
};
export declare function weierstrass(c: CurveType): CurveFn;
//# sourceMappingURL=weierstrass.d.ts.map
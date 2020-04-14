# Contributing to Tree-Buf
We're stoked that you would like to contribute to Tree-Buf! Please refer to the following guidelines before getting started.

## What kind of changes are welcome?
Until Tree-Buf matures care must be taken to ensure that changes are directed toward Tree-Buf's short-term goals. It may be best to [open an issue](https://github.com/That3Percent/tree-buf/issues) before submitting a PR to sync up on whether a change is a good idea _right now_ before the format is stabilized.

For example, porting Tree-Buf to a second language would not be a good investment of time because until the format is stabilized it will be difficult to keep multiple implementations in sync.

When maturity is reached it is expected that a broader range of contributions will be welcome.

## Performance improvements

All performance improvements must be accompanied by benchmarks using real-world data sets. A separate repository will be used for storing benchmarks (coming soon)

## Bugfixes

All bugfixes must be accompanied by a test case which failed before the bugfix and passes with the bugfix.

## Unsafe
Unsafe code is allowed, especially for performance reasons. It must be sound. If I'm not completely convinced of it's soundness, it will be assumed to be unsound.

## Contributions
All code in this repository is under the [MIT](http://opensource.org/licenses/MIT) license.
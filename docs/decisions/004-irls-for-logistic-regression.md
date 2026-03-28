# 4. IRLS for Logistic Regression

Date: 2026-03-28
Status: Accepted

## Context

Logistic regression requires an iterative solver. The two practical choices are:

1. **Gradient descent (or SGD)** -- simple to implement but requires a learning
   rate hyperparameter, is sensitive to feature scaling, and converges linearly.
2. **Iteratively Reweighted Least Squares (IRLS / Fisher scoring)** -- a
   Newton-Raphson variant that converges quadratically and requires no learning
   rate.

Pramana already depends on hisab for linear algebra, which provides Cholesky
decomposition -- exactly what IRLS needs for its inner solve step.

## Decision

Use IRLS (Newton-Raphson / Fisher scoring) with:

- **L2 regularization** (ridge penalty) to handle near-separable data.
- **Cholesky solve** via hisab for the normal equations at each iteration.
- **Convergence criterion** based on relative change in log-likelihood.
- **Maximum iteration cap** (default 25) as a safety bound.

## Consequences

**Positive**

- No learning rate parameter -- one fewer knob for users to tune.
- Quadratic convergence means fewer iterations (typically 4--8).
- Reuses hisab's existing Cholesky infrastructure.

**Negative**

- Requires regularization for perfectly separable datasets (without it, the
  Hessian becomes singular). L2 is always applied with a configurable lambda.
- Memory cost is O(p^2) for the Hessian where p is the number of features. This
  is acceptable for the problem sizes pramana targets.

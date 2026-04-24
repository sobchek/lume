::  lib/vesl-verifier.hoon: STARK proof verification for Vesl computations
::
::  The standard nock-verifier calls puzzle-nock to reconstruct [s f],
::  which fails for non-puzzle computations.  This verifier provides:
::
::  Level 1 (verify-structure): structural + re-execution check.
::  Level 2 (verify): full STARK math via vesl-stark-verifier fork.
::
/=  *  /common/zeke
/=  vesl-stark-verifier  /lib/vesl-stark-verifier
/#  softed-constraints
::
|%
::
::  +verifier: STARK verifier initialised with softed constraints
::
::  Mirrors nock-verifier.hoon pattern: inject stark-config
::  into the vesl-stark-verifier door via +<+< axis.
::
++  verifier
  =|  in=stark-input
  =/  sc=stark-config
    %*  .  *stark-config
      prep  softed-constraints
    ==
  %_    vesl-stark-verifier
      +<+<
    %_  in
      stark-config  sc
    ==
  ==
::
::  +verify: full STARK math verification
::
::  Accepts [s f] directly instead of deriving from puzzle-nock.
::  All FRI, linking-checks, constraint satisfaction, and DEEP
::  polynomial checks execute identically to the standard verifier.
::
++  verify
  |=  [=proof override=(unit (list term)) eny=@ s=* f=*]
  (verify:verifier proof override eny s f)
::
::  +verify-raw: debug variant of +verify that does NOT swallow
::  verify-inner crashes with `mule`. Internal `?>` failures bubble
::  up as kernel crash traces so app integrators can see which
::  assertion rejected a proof. See `vesl-stark-verifier.hoon` for
::  details — the `softed-constraints` injection is identical to
::  `+verify`.
::
++  verify-raw
  |=  [=proof override=(unit (list term)) eny=@ s=* f=*]
  (verify-raw:verifier proof override eny s f)
::
::  +verify-structure: structural + re-execution validation
::
::  Re-executes [subject formula] and compares product against
::  the product embedded in the proof's %puzzle entry.
::
++  verify-structure
  |=  [prf=proof subject=* formula=*]
  ^-  ?
  ::  1. version must be %2
  ?.  ?=(%2 version.prf)
    %.n
  ::  2. proof must have objects
  ?~  objects.prf
    %.n
  ::  3. first object must be %puzzle
  =/  first=proof-data  i.objects.prf
  ?.  ?=(%puzzle -.first)
    %.n
  ::  4. re-execute and compare product
  =/  result  (mule |.(.*(subject formula)))
  ?.  ?=(%& -.result)
    %.n
  =(p.result p.first)
::
::  +extract-product: get computation product from a proof
::
++  extract-product
  |=  prf=proof
  ^-  (unit *)
  ?~  objects.prf  ~
  =/  first=proof-data  i.objects.prf
  ?.  ?=(%puzzle -.first)  ~
  `p.first
--

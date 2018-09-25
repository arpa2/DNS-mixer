# Integrating min(DNS) and max(DNS) into the DNS mixer

> *Many precautions in DNS, specifically when dealing with DNSSEC and monitoring,
> rely on precise understanding of caching during the TTL of (retracted) resource
> records.  Thorough implementations of these disciplines involves performing
> timing computations of staggering complexity.  With min/max DNS, a much simpler
> tool is available.  We explain the tool below, as well as its integration with
> the DNS mixer.*

The purpose of min/max(DNS) is to represent cache behaviour as a variant of
the Current DNS as it is output by authoritative DNS servers.  Caches are a
realistic part of concerns around DNS changes, especially in relation to
service reliability.

Some algorithms require for their robustness that every client of a DNS
zone has access to a certain resource record; other algorithms require
for their robustness that no client of a properly handled DNS zone
has access to resource records anymore.

This is addressed through min/max(DNS).  To define the two components:
min(DNS) is the set of resource records known to be available everywhere,
and max(DNS) is the set of resource records that may be available somewhere.
This implements standard caching rules under the assumption that
DNS implementations adhere to them or otherwise are left to their own
devices when they suffer from spurious events.

The implementation detailed below extends the DNS mixer with information
alongside each resource record to define its latest time of introduction
and, once removed, its latest time of retraction.  This captures the
impact of caches: delays in the introduction of new data, and in the
removal of old data.


## Relation to IXFR

We shall assume that Current DNS information is provided from a
(partial) master using `IXFR`.  When an `AXFR` comes in, it is to be
compared with prior zone data.  When no prior zone data is available,
it must be assumed that the prior data was an empty zone; this
may lead to an initial underestimation of the max(DNS) data, and
cautious underestimation of min(DNS).

A subscription to the (partial) master zone's data is vital, to
have accurate timing of any changes to resource records.  In
other words, the computation of min/max(DNS) is only possible on
slave DNS servers.

The `IXFR` message adds some resource records and removes others.
As a special case, it may modify the TTL of a resource record but
leave it otherwise unchanged.  The TTL values are important, along
with the possibility that a value is absent, in which case the
TTL for negative caching may have to be considered.  In short, there
is a lot of complexity that can be solved once and for all with a
facility for min/max(DNS).  All this can be done on the basis of
the information in an `IXFR` message (and by referencing the zone
data for Current DNS).


## min(DNS) and max(DNS) in the DNS mixer

The DNS mixer is designed to selectively passing resource records, possibly
modifying resource records before adding them to output zones.  Some of the
operations involve changing records, for instance from `DNSKEY` to `CDNSKEY`
or `CDS`, which is unsafe unless all caches are known to hold the `DNSKEY`.
This means that a safe ACL Rule should be based on min(DNS), rather than on
Current DNS.

When storing the partial master data, each resource record is annotated with
a latest time of introduction and a latest time of retraction.  The latest
introduction time will be computed when adding a resource record; the latest
retraction time is initially unset and will be updated when the resource
record is removed.

The `ttl` field in an ACL Rule looks at values of TTL fields, but this
is concerned with their current value; min/max(DNS) is more concerned with
their development in historic context, and extends the field accordingly.
ACL Rules may add a marking to the `ttl` field to indicate the use of
min(DNS) or max(DNS) instead of the default, Current DNS.

    ttl max   # Use max(DNS)
    ttl min   # Use min(DNS)
    ttl       # Current DNS

The `max` and `min` word feel like mere selections on the data, which
is indeed the intended level of conceptual simplicity to be provided
by min/max(DNS).  What happens under the hood is a bit more complex,
but not necessarily too disturbing for operators.  The combined use
of `min` and `max` is meaningless.

When looking at Current DNS, the mere existence of a resource record allows
the ACL Rule to proceed.  This is the straightforward case that was
originally designed into the DNS mixer.

When relying on max(DNS), any removal of a resource record through the
ACL Rule will not commence until the latest time of retraction in some
cache somewhere on the Internet, but additions are instantly processed
because there can be a cache that picks it up immediately.

When relying on min(DNS), any addition of a resource record through the
ACL Rule will not commence until the latest time of introduction to
represent the slowest adoption by some cache somewhere, but
removal is instantly processed because it might happen to be noticed
immediately by some cache somewhere.  min(DNS) makes it possible for
removal to overtake addition, in which case the two operations
cancel against each other, and no action will occur due
to the ACL Rule for that resource record.

When starting the DNS mixer, it is important that any delayed actions
of ACL Rules are recreated, possibly by triggering them once more from
the input in partial master's zones.  Some of the work may take place
immediately as a result of downtime of the DNS mixer.  Some mechanism
will be needed to determine the time up
to which ACL Rule processing has been completed and output; this can be
incorporated into the same transaction that is involved in the data
stored for input and output zones.  A timestamp related to the last
output zone update is probably a good source to determine what
delayed actions have taken place at the time of a restart.

Given that ACL Rules need to defer actions, it is possible to
allow safety margins on the timing.  This is indeed a common
practice in everyday DNS.  So, we allow extensions such as

    ttl max+3600
    ttl min-3600

These settings add a safety margin of 3600 seconds to the timing
of min/max(DNS).  The sign of `+` or `-` is immaterial and in fact
they are interchangeable.  It is only possible to increase safety,
because anything else would be senseless.  So, we do not have to
confuse operators by enforcing a `+` or `-` in either place.

Safety for min(DNS) means that the extra time is added to the time
of earliest introduction, effectively assuming a possible lag time
or clock abberation and maybe some TTL totalling up to 3600 seconds.
This means that procedures that ensure that a resource
record is available everywhere err on the safe side.

Safety for max(DNS) means that the extra time is added to the time
of last retraction, again under the assumption of lag time,
clock abberation and TTLs.  This means that procedures that ensure
that a resource record is absent everywhere err on the safe side.

Note that the safety margins apply only on one end.  The other
end might be influenced by setting them on related resource
records in the opposite manner.  Depending records might choose
`min-` notation while dependend-on records would use `max+` to
extend their visible life.

### A few Finer Points

A system like this, retro-fitted onto something as pragmatic as
DNS, ends up making a few choices at a detail level.  Not many,
as it happens, but still a few.

One fine point is what TTL is used in the computations of `min`
and `max`; since these may be altered by an ACL Rule, this is a
valid concern.  The choice made here is to maintain consistency
with the original data received from the partial master.  When
an ACL Rule modifies the TTL it is advised to include a safety
margin into the `min` or `max` declaration to compensate.

Another fine point is that an ACL Rule matches resource records
on partial information only, and without concerns for other
resource records.  This should be the intention of the operator
who specified the patterns, as these are usually not of interest
when they spell out every bit of the resource data.  The trick
is to find precisely those patterns for the ACL Rule that match
the cases of interest.


## Operations with min/max(DNS)

The various changes that occur in a zone give rise to processing
rules.  Given that DNS is more often pragmatic than consistent in
its rules, the discussion below dissects the various cases and
details the desired actions separately.

In general, these operations are combined into as a transaction,
involving the `IXFR` change to the input zone from the given partial master
joint with all changes to output zones.  Zones should hold back
any downstream change notifications or upstream acknowledgements
as long as the transaction has not been committed.


### Introducing a New Resource Record Set

The first case introduces a resource record for which no current
records with the same owner name and resource record type were
known.  Note the reference to Current DNS; older resource records
whose validity may not yet have expired are to be treated
orthogonally; they may be integrated and their TTL set apropriately
in the zone output phase.

Whether a newly added resource record actually starts a new resource
record set can be observed by looking at the prior version for
the partial master zone to which the change was submitted; it cannot
be inferred from the `IXFR` alone because that would only mention
resource records that have changed within a larger set.

These new resource record sets are setup with resource records
that each have a latest introduction time being the time of
arrival of the update, plus the negative caching time for the zone.
According to RFC 1035, this negative caching time is the minimum
of the `SOA` minimum value and the `SOA` TTL; these values are
accessible in the `IXFR` and should be taken from the old `SOA`
as that determines any negative caching time prior to the
publication of the new resource record set.  (A truly correct
implementation would trace past `SOA` values up to the prior
one, and summerise the latest possible negative caching TTL
plus the time of the update that outdated the containing `SOA`.
A pragmatic implementation could get away with only observing
the prior `SOA` and would hardly ever be wrong.)


### Retracting a Resource Record Set

The complete removal of a resource record set means that all
current resource records for a given owner name and resource
record type are removed by an update.  This references Current
DNS, as any older resource records should be treated orthogonally
and may at best mix in due to their TTL in the zone output stage.

The TTL of the removed resource record set should be added to
the current time to find the time of last retraction for each
of the resource records being removed.


### Changing the TTL of a Resource Record Set

Even the most elaborate of procedures that check for certain
presence or absense of information in DNS make one pragmatic
choice, and that is to forego changes in TTL of resource
records in the reasoning.  But because of real-time observation
of the flow of DNS changes, this is not necessarily a problem
for min/max(DNS).

A problematic scenario would be a TTL that changes from 7200
to 3600, quickly followed by a change to a record that is
deferred until that new TTL has expired.  This may be part of
an inaccurate assumption that the TTL has expired, and that
everyone now sees the new resource record.  In reality, there
may still be caches that hold the value based on the TTL
of 7200 seconds.

There are a few ways of dealing with this, such as internal
count-down of TTL fields from the old to the new value.
Given the design of the DNS mixer with its multiset
support however, we can just keep the old and new TTL
both activated, at least until the old TTL has expired.
In this case, the output zone will hold the values at
least as long as the original publication indicated, and
any further changes to the resources would not be confused.

So, when the TTL of a resource record set changes, its
old resource records are treated as though they are being
retracted, setting a suitable time for last retraction.
At the same time, the ones with the new TTL are treated like
they were introduced, though the earliest introduction time
for these new records should be set to either the current
time or the earliest introduction time for the old resource
records, whichever is later.


### Adding to a Resource Record Set

When a new resource record is added to an already existing
set, and when the TTL matches that of the existing set,
then a simpler operation can commence.

In this case, the record is added with its time of earliest
introduction set to the current time plus a delay.  This
delay can the current time plus the negative caching time for
young records, or it can be the current time plus the TTL of
the already-existing resource records in the same set.
In general, the earliest introduction time is set to the
latest of these times, which effectively disables the former
value for anything but the youngest records.

Note how this computation overlaps that for a completely
new resource record set, with the exception that there
is no time for the already-existing resource records when
no such records exist, so the current time plus
negative caching is then automatically the only option.
A general procedure covering both forms of added
resource records might be to first establish the current
time plus negative caching time, and then to iterate the
already-existing resource records in the same set, allowing
them to increase the earliest introduction time.


### Removing from a Resource Record Set

When a resource record is removed, but other elements of
the resource record set remain, then the
removed resource records need to have their time of
latest retraction set.  This will be the current time plus
the TTL for the resource record.

Note that the mere setting of the time of latest retraction
is indicative of a resource record that has been removed
from Current DNS.  So, when looking for entries belonging
in Current DNS, the time of latest retraction must be
unset.

Note how this behaviour coincides with the complete
removal of a resource record set; this means that a simpler
implementation is possible, without checking if any
resource records would remain.


## Export Quality max(DNS)

The logic based on a latest retraction time suffices for
the timing of a ACL Rule with `ttl max`, but it does not
provide enough information to be able to export DNS information
that represents max(DNS) to external requesters.  Such
requesters may see sudden drops, without any anticipation
of when this would happen, and so they are left to polling
and, when speed is important, doing so at a much higher
rate than the TTL.  This is not very handy, and a TTL that
counts down as it does in a cache would be a more useful
instrument in max(DNS), as it would allow setting a timer
for a following check.  This timer would run once in a
TTL and potentially much faster.  Note that the logic of
`RRSIG` allows lower TTL values than used while signing,
in support of cached resource records, see
[Section 3.1.4 of RFC4034](https://tools.ietf.org/html/rfc4034#section-3.1.4).

Now, instead of presenting a resource record as-is even
if its removal is scheduled, it is possible to use the
latest retraction timer if it is set.  The number of
seconds from the query time until the latest retraction
time suffices to determine a TTL to send out.  When no
removal is scheduled, the TTL can be sent out as-is;
removal is then the time when down-counting starts.
**TODO:** Can we incorporate a safety margin?  It would
be best not to exceed the previous TTL.

This is not a solution for a drop in TTL for the same
resource record.  When the old TTL is counted down
until removal, its low value would overrule other
resource records with the new TTL, so that is not
perfect.  Instead, we can decide to count down the TTL
until the new value was reached.  Similar to the idea
of counting down to a latest retraction time, we could
count down to a new TTL value.  This can be integrated
with the retraction time by setting the new TTL value
to 0 and speaking of a latest countdown time to reach
a TTL.  Removal is possible when zero is reached.  (A
remaining fine point might be the distinction from a
resource record that is published with TTL 0, which
should not be the result of counting down but then
again it is also not meant for caching.  Dropping a
TTL from a higher value to 0 is certainly silly.)

When a drop to an even lower TTL follows the previous
one, then the new TTL may be set with another TTL and
corresponding latest countdown time, but the speed of
counting down the original records does not go up, so
it's all part of the same line going down.

We do not need to count gradually when incrementing a
TTL value.  A higher value for the TTL should be set
immediately, to reflect that some caches might find it
right away.  When a resource record is counting down
to a lower TTL, we should look at its currently
reported TTL instead of the TTL field itself, and if
the new TTL is at least as high it should be set
immediately and the countdown stopped.  When the
countdown works towards a lower TTL but it has not
dropped as far yet as the newly proposed TTL value,
then the countdown should be modified to stop when
the proposed new TTL is reached.

The result is that we can compute the maximum TTL
that a resource record could have in some cache
somewhere, and this means that we can predict when
cache entries for the resource record expire.  The
resulting time is a perfect time for polling a
max(TTL) name server once more, to see if the state
of DNS has changed or that we can proceed under the
assurance that nodody is still using the requested
resource record.

The TTL computations in max(TTL) are made for each
resource record individually.  When adding or
removing, this can lead to variations in TTL within
a resource record set as published by max(TTL).
Even if this is deprecated behaviour in Current DNS
for lack of application, it is the most informative
that max(DNS) can be and should not be overruled for
its output.


## Export Quality min(DNS)

The purpose of min(DNS) is to test if certain
resource records are available everywhere.  This
means that the value of the TTL is not important;
we are not waiting for the information to disappear
and we need to do no polling when something appears
in min(DNS).

Setting the lowest TTL may sound like it provides
useful information, and could help polling such as
for monitoring, but this is actually wrong.  As
soon as Current DNS drops a resource record, it
also drops out of min(DNS).  So the only safe TTL
for min(DNS) would be zero, which is intended for
the reporting of information that must not be
cached.

With that in mind, the setting of a
latest introduction time is sufficient for the
behaviour of min(DNS) to be of export quality.
Removal of a resource record from a min(DNS) flow
should of course be immediate.  When one
integrated administration is used this may require
a flag to be added to indicate that a record has
been removed.  This also resolves the question on
what to do when a counting-down zone reaches 0
in the most stable manner possible.

## Export Quality Operations with min/max(DNS)

The refined max/min(DNS) model calls for different
procedures to process `IXFR` changes.  In fact, we
can store more information and might find a simpler
model.  We now store the following alongside a
resource record:

  * A time of latest introduction
  * A TTL value, or the target of optional countdown
  * An optional time at which TTL countdown finishes
  * A flag to indicate removal from authoritatives

For normal DNS service, we would only show resource
records that have no removal flag, and we would use
the TTL value as set, because that is actually the
authoritative TTL value, even if max(TTL) intends to
be clever around it.  This TTL value may therefore
be set in the resource record itself.

Only the operations that have changed are presented
below; the others remain unchanged.

### Changing the TTL of a Resource Record Set

The change of a TTL on a resource record set is the
most interesting refinement to make with the new
data being stored.  We are going to iterate over
independent resource records.

Some resource records may be added or dropped in
an `IXFR`; there are procedures for that below.
Other resources records will not change, except
for their TTL.  This is of no impact on min(DNS)
but it does influence max(DNS).  In other words,
we should follow the rules for Export Quality
for max(DNS), leading to possible updates of the
TTL value and the optional TTL countdown time.

This demonstrates that the new data gives a finer
representation than before, where we resorted to
the removal of a resource record and the introduction
of another.  Though this worked for internal use, it
might send off more signals than desired.

### Removing from a Resource Record Set

To remove a resource record from a set, its deletion
flag is set.  Historic records may still exist with
this flag set, but those should be skipped, to avoid
problems when resource records are flapping.

To handle max(DNS), we count down from the TTL to 0,
so we set the TTL to 0 and countdown end time to the
current time plus the previous TTL value.  If a
countdown was already in progress, we should instead
add the old TTL to the already-set countdown time and
then set the TTL to 0.

Current DNS and min(DNS) will stop servicing the
resource record immediately; max(DNS) will continue
to serve it until the TTL reaches 0, at which time
the deleted flag states that it is up for removal.

### Retracting a Resource Record Set

The removal of a resource record set consists just of
the removal of the individual resource records in it.
This is a pleasant change; we have no need to see if
resource records remain if an `IXFR` orders removal of
some.


## Example

It is not safe to copy a `DNSKEY` record to a parent as a `DS`
record, or take the corresponding actions of adding a `CDS` or
`CDNSKEY` record to a zone.  This however, is safe when used
with min(DNS).

So, it would be unsafe to specify

    name **.__ ^2 =2 ; type RRSIG              ; ttl
    name **.__ ^2 =2 ; type NSEC3              ; ttl
    name **.__ ^2 =2 ; type DNSKEY             ; ttl
    name **.__ ^2 =1 ; type DNSKEY dnskey2ds 2 ; ttl

But it would be safe to specify

    name **.__ ^2 =2 ; type RRSIG              ; ttl max
    name **.__ ^2 =2 ; type NSEC3              ; ttl max
    name **.__ ^2 =2 ; type DNSKEY             ; ttl max
    name **.__ ^2 =1 ; type DNSKEY dnskey2ds 2 ; ttl min

Or perhaps add some safety margin with

    name **.__ ^2 =2 ; type RRSIG              ; ttl max+15
    name **.__ ^2 =2 ; type NSEC3              ; ttl max+15
    name **.__ ^2 =2 ; type DNSKEY             ; ttl max+15
    name **.__ ^2 =1 ; type DNSKEY dnskey2ds 2 ; ttl min-15

The most important line in these fragments is the last; the
explicit mention of `RRSIG`, `NSEC3` and `DNSKEY` shows how
these records' visibility may also be extended in the safe
direction.  These records will show the usual TTL value,
but the record will be shown longer than the input from the
partial master zone.

The notation `**._ ^2` is a generic pattern to cover names
that fall one level below a TLD delegation point.  The
variation with `__` instead of `_` works similarly, but
takes locally known zone apexes and/or `NS` references
into account for defining delegation points.

By default, the `ttl` is reduced to within certain sensible
bounds.  But with min/max(DNS) as annotations on the `ttl`
field, these defaults do not apply.  DNS allows times being
specified from 1 second up to 136 years, so it can be
sensible to repeat such constraints here, either clipping
the extremes or forbidding outliers.

This is admittedly a bit complex.  But in comparison to a
stepwise procedure that repeatedly looks in DNS at several
locations and enforces safety that way, it really is
child's play.

Something not covered here, but certainly useful, is any
delay to `DS` processing in an external parent zone.  It
may be necessary to continue publishing DNSSEC data in a
zone for the publication delay plus the TTL of the parent
zone for these `DS` records.



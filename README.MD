README for DNS mixer
====================

> *"DNS mixers" are a new kind of component for the DNS infrastructure.
> They accept input from hidden masters that have only a partial
> understanding of what data should be sent to a zone.  The DNS mixer
> is a slave to such "partial master" and produces zones holding their
> mixed zone data.  The DNS mixer enforces access control to the zone
> data published for each of the partial masters.*

This is currently an idea for a project, described in a design document.
We intend to have it built soon, as we need it for two projects that we
have coming up:

  * Support for our future
    [ServiceHub phase](http://internetwide.org/blog/2016/06/24/iwo-phases.html)
    in the
    [InternetWide Architecture](http://internetwide.org/tag/architecture.html)
    which aims to provide
    [a new deal for hosting providers](http://internetwide.org/blog/2014/11/19/back-to-hosting.html)
    including facilitation of our realm-crossing concept of
    [Bring Your Own IDentity](http://internetwide.org/blog/2015/04/22/id-2-byoid.html).

  * Support for vendor-supplied `NAPTR` records in
    [Public User ENUM](http://internetwide.org/blog/2014/11/21/telephony-emancipation.html)
    which we consider a valid and useful
    [form of user identity](http://www.internetwide.org/blog/2016/12/30/id-9-enum-xmpp.html)
    to which we intend to grant
    [control by users](http://internetwide.org/blog/2014/11/26/online-identity.html).

There are a few other applications that could benefit from the idea of a
DNS mixer:

  * Dynamic DNS systems that set host names under an otherwise normal zone.
    When [externally provided](https://www.nsupdate.info) for, access control
    after proper authentication is needed to avoid overwriting zone data.

  * Multicast DNS systems may use a similar mechanism to integrate parts of
    a zone to form a larger zone, taken from individual `.local` entries.
    This may lead to zone data that is as pluggable as the network itself.

  * Peer-to-peer networks holding DNS data.  Note that DNSSEC is usually
    a good idea in such networks, except that NSEC and NSEC3 are unlikely
    to work, so timeouts may slow down the network operation.

  * Migrations between DNSSEC signing solutions, such as the crossover between
    DNS service providers or changing from one HSM-based signer to another.
    Special precautions are needed to do this, such as the introduction of the
    zone public keys from one side to the other side, but these tasks as well
    as the delivery of the signed output from old and new situation may well
    be setup with a DNS mixer.


#!/usr/bin/env python3
"""Offline reference evaluator for EgoWitness-style spatial identity fixtures.

Metadata-only benchmark tooling; not a production tracker, SLAM system,
biometric recognizer, map builder, or memory engine.
"""
from __future__ import annotations
import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

SCHEMA_VERSION=1
BENCHMARK_ID="ego-witness-spatial-memory-v0"


class EvaluationError(ValueError):
    """Malformed benchmark inputs."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r",encoding="utf-8") as handle:
        value=json.load(handle)
    if not isinstance(value,dict):
        raise EvaluationError(f"{path}: root must be object")
    return value


def ratio(n:int,d:int)->float|None:
    return n/d if d else None


def _index(rows:list[dict[str,Any]],key:str)->dict[str,dict[str,Any]]:
    out={}
    for row in rows:
        value=row.get(key)
        if not isinstance(value,str) or not value or value in out:
            raise EvaluationError(f"missing or duplicate {key}: {value!r}")
        out[value]=row
    return out


def _pair_key(left:str,right:str)->tuple[str,str]:
    if left==right:
        raise EvaluationError("pair endpoints must be distinct")
    return tuple(sorted((left,right)))


def _confusion(truth:dict[tuple[str,str],bool],predicted:dict[tuple[str,str],bool]):
    tp=fp=tn=fn=correct=0
    for key,label in truth.items():
        value=predicted.get(key)
        if value is None:
            if label:
                fn+=1
            continue
        if value==label:
            correct+=1
        if value and label: tp+=1
        elif value and not label: fp+=1
        elif not value and label: fn+=1
        else: tn+=1
    return {
        "count":len(truth),"correct":correct,"accuracy":ratio(correct,len(truth)),
        "tp":tp,"fp":fp,"tn":tn,"fn":fn,
        "precision":ratio(tp,tp+fp),"recall":ratio(tp,tp+fn),
        "false_merge_count":fp,"false_split_count":fn
    }


def _validate_episode(episode:dict[str,Any])->dict[str,dict[str,Any]]:
    query=episode.get("query") or {}
    query_tick=query.get("query_tick")
    if not isinstance(query_tick,int) or isinstance(query_tick,bool):
        raise EvaluationError(f"{episode.get('episode_id')}: invalid query tick")
    observations=episode.get("observations")
    if not isinstance(observations,list) or not observations:
        raise EvaluationError(f"{episode.get('episode_id')}: observations must be non-empty array")
    indexed={}
    ticks=[]
    local_tracks=set()
    for row in observations:
        ref=row.get("observation_ref")
        if not isinstance(ref,str) or not ref or ref in indexed:
            raise EvaluationError(f"{episode.get('episode_id')}: missing or duplicate observation_ref")
        for field in ("availability_tick","capture_tick","clock_generation"):
            if not isinstance(row.get(field),int) or isinstance(row.get(field),bool):
                raise EvaluationError(f"{ref}: invalid {field}")
        tick=row["availability_tick"]
        if tick>query_tick or row["capture_tick"]>tick:
            raise EvaluationError(f"{ref}: observation is unavailable at query or capture follows arrival")
        ticks.append(tick)
        track=row.get("local_track_ref")
        if not isinstance(track,str) or not track or track in local_tracks:
            raise EvaluationError(f"{ref}: local track refs must be unique and non-empty")
        local_tracks.add(track)
        if not isinstance(row.get("artifact_sha256"),str) or len(row["artifact_sha256"])!=64:
            raise EvaluationError(f"{ref}: invalid artifact digest")
        indexed[ref]=row
    if ticks!=sorted(ticks):
        raise EvaluationError(f"{episode.get('episode_id')}: observation availability order is not monotonic")
    if query.get("response_deadline_tick",query_tick)<query_tick:
        raise EvaluationError(f"{episode.get('episode_id')}: response deadline precedes query")
    return indexed


def evaluate(workload:dict[str,Any],truth:dict[str,Any],submission:dict[str,Any])->dict[str,Any]:
    for label,doc in (("workload",workload),("truth",truth),("submission",submission)):
        if doc.get("schema_version")!=SCHEMA_VERSION or doc.get("benchmark_id")!=BENCHMARK_ID:
            raise EvaluationError(f"{label}: schema or benchmark id mismatch")
    wrows,trows,prows=workload.get("episodes"),truth.get("episodes"),submission.get("predictions")
    if not all(isinstance(x,list) for x in (wrows,trows,prows)):
        raise EvaluationError("episode/prediction arrays missing")
    w=_index(wrows,"episode_id"); t=_index(trows,"episode_id"); p=_index(prows,"episode_id")
    ids=set(w)
    if set(t)!=ids or set(p)!=ids:
        raise EvaluationError("workload, truth and candidate episode IDs must match exactly")

    episode_results=[]
    all_instance_truth={}; all_instance_predictions={}
    all_room_truth={}; all_room_predictions={}
    citation_tp=citation_pred=citation_required=0
    answer_correct=decision_correct=coverage=provenance_count=qualified_count=0
    last_seen_count=last_seen_correct=false_wearer=0
    violations_total=0

    for episode_id in sorted(ids):
        episode,label,pred=w[episode_id],t[episode_id],p[episode_id]
        observations=_validate_episode(episode)
        query=episode["query"]; query_tick=query["query_tick"]
        violations=[]
        if pred.get("decision") not in {"answer","abstain"}:
            violations.append("invalid_decision")
        abstained=pred.get("decision")=="abstain"
        answer=pred.get("answer")
        conf=pred.get("confidence")
        if isinstance(conf,bool) or not isinstance(conf,(int,float)) or not math.isfinite(float(conf)) or not 0<=float(conf)<=1:
            violations.append("invalid_confidence")
            conf=0.0
        if abstained and answer is not None:
            violations.append("abstention_contains_answer")
        if not abstained and (not isinstance(answer,str) or not answer):
            violations.append("answer_decision_requires_answer")
        cutoff=pred.get("evidence_as_of_tick")
        response=pred.get("response_tick")
        if cutoff!=query_tick:
            violations.append("evidence_cutoff_must_equal_query_tick")
        if not isinstance(response,int) or isinstance(response,bool) or response<query_tick:
            violations.append("invalid_response_tick")
        elif response>query["response_deadline_tick"]:
            violations.append("response_after_deadline")

        citations=pred.get("cited_observation_refs",[])
        if not isinstance(citations,list) or any(not isinstance(x,str) for x in citations):
            citations=[]
            violations.append("invalid_observation_citations")
        if len(set(citations))!=len(citations):
            violations.append("duplicate_observation_citations")
        if any(ref not in observations for ref in citations):
            violations.append("citation_not_in_episode_or_not_admitted")
        admitted_citations=[ref for ref in citations if ref in observations]
        required=set(label.get("required_observation_refs",[]))
        if not required<=set(admitted_citations):
            violations.append("required_source_citation_missing")
        citation_tp+=len(required & set(admitted_citations))
        citation_pred+=len(admitted_citations)
        citation_required+=len(required)
        citation_precision=ratio(len(required & set(admitted_citations)),len(admitted_citations))
        citation_recall=ratio(len(required & set(admitted_citations)),len(required))
        provenance_complete=required<=set(admitted_citations) and not any(
            ref not in observations for ref in citations
        )

        answer_ok=(not abstained) and answer==label.get("expected_answer")
        expected_decision=label.get("expected_decision","answer")
        decision_ok=(abstained if expected_decision=="abstain" else (not abstained and answer_ok))
        if abstained and expected_decision!="abstain":
            decision_ok=False
        answer_correct+=int(answer_ok); decision_correct+=int(decision_ok); coverage+=int(not abstained)

        # Identity-pair claims: exact pair set must be supplied once.
        instance_truth={}
        for row in label.get("instance_pair_truth",[]):
            key=_pair_key(row["left_observation_ref"],row["right_observation_ref"])
            if key in instance_truth:
                raise EvaluationError(f"{episode_id}: duplicate instance truth pair")
            instance_truth[key]=row["same_instance"]
        instance_predictions={}
        raw_ip=pred.get("instance_pair_predictions",[])
        if not isinstance(raw_ip,list):
            raw_ip=[]
            violations.append("invalid_instance_pair_predictions")
        for row in raw_ip:
            try:
                key=_pair_key(row.get("left_observation_ref"),row.get("right_observation_ref"))
                same=row.get("same_instance")
                if not isinstance(same,bool):
                    raise EvaluationError("same_instance must be boolean")
                if key in instance_predictions:
                    violations.append("duplicate_instance_pair_prediction")
                instance_predictions[key]=same
                if not set(key)<=set(observations):
                    violations.append("instance_pair_references_unadmitted_observation")
                if not set(key)<=set(admitted_citations):
                    violations.append("instance_pair_evidence_not_cited")
            except (TypeError,EvaluationError):
                violations.append("malformed_instance_pair_prediction")
        if set(instance_predictions)!=set(instance_truth):
            violations.append("instance_pair_prediction_set_incomplete_or_extra")
        for key in instance_truth:
            if key not in instance_predictions:
                all_instance_predictions[(episode_id,)+key]=None
            else:
                all_instance_predictions[(episode_id,)+key]=instance_predictions[key]
            all_instance_truth[(episode_id,)+key]=instance_truth[key]

        # Room identity claims use the same observation anchors but a separate relation.
        room_truth={}
        for row in label.get("room_pair_truth",[]):
            key=_pair_key(row["left_observation_ref"],row["right_observation_ref"])
            if key in room_truth:
                raise EvaluationError(f"{episode_id}: duplicate room truth pair")
            room_truth[key]=row["same_room"]
        room_predictions={}
        raw_rp=pred.get("room_pair_predictions",[])
        if not isinstance(raw_rp,list):
            raw_rp=[]
            violations.append("invalid_room_pair_predictions")
        for row in raw_rp:
            try:
                key=_pair_key(row.get("left_observation_ref"),row.get("right_observation_ref"))
                same=row.get("same_room")
                if not isinstance(same,bool):
                    raise EvaluationError("same_room must be boolean")
                if key in room_predictions:
                    violations.append("duplicate_room_pair_prediction")
                room_predictions[key]=same
                if not set(key)<=set(observations):
                    violations.append("room_pair_references_unadmitted_observation")
                if not set(key)<=set(admitted_citations):
                    violations.append("room_pair_evidence_not_cited")
            except (TypeError,EvaluationError):
                violations.append("malformed_room_pair_prediction")
        if set(room_predictions)!=set(room_truth):
            violations.append("room_pair_prediction_set_incomplete_or_extra")
        for key,value in room_truth.items():
            all_room_truth[(episode_id,)+key]=value
            all_room_predictions[(episode_id,)+key]=room_predictions.get(key)

        # Last-seen claims are checked against hidden stable instance identity and as-of availability.
        last_truth=label.get("last_seen_truth")
        last_pred=pred.get("last_seen_prediction")
        last_seen_ok=None
        if last_truth is None:
            if last_pred is not None:
                violations.append("unexpected_last_seen_claim")
        else:
            last_seen_count+=1
            if not isinstance(last_pred,dict):
                violations.append("last_seen_prediction_missing")
            else:
                target=last_pred.get("target_observation_ref")
                selected=last_pred.get("selected_observation_ref")
                wearer=last_pred.get("wearer_ref")
                mapping=label.get("instance_identity_by_observation",{})
                target_instance=mapping.get(last_truth["target_observation_ref"])
                eligible=[
                    (ref,row) for ref,row in observations.items()
                    if mapping.get(ref)==target_instance and row["availability_tick"]<=query_tick
                ]
                expected_latest=max(eligible,key=lambda pair:(pair[1]["capture_tick"],pair[1]["availability_tick"]))[0] if eligible else None
                if target!=last_truth["target_observation_ref"]:
                    violations.append("last_seen_target_identity_mismatch")
                if selected not in observations:
                    violations.append("last_seen_selected_observation_unadmitted")
                if selected in observations and selected not in admitted_citations:
                    violations.append("last_seen_evidence_not_cited")
                if selected!=expected_latest:
                    violations.append("last_seen_not_latest_eligible_instance_observation")
                selected_row=observations.get(selected)
                if selected_row is not None and wearer!=selected_row["wearer_ref"]:
                    false_wearer+=1
                    violations.append("last_seen_wearer_attribution_mismatch")
                last_seen_ok=(
                    target==last_truth["target_observation_ref"]
                    and selected==last_truth["expected_observation_ref"]
                    and wearer==last_truth["expected_wearer_ref"]
                    and selected==expected_latest
                )
                if not last_seen_ok:
                    violations.append("last_seen_answer_incorrect")
            if not answer_ok:
                pass
            elif last_seen_ok is False:
                pass
            if last_pred is not None:
                for ref in (last_pred.get("target_observation_ref"),last_pred.get("selected_observation_ref")):
                    if ref not in admitted_citations:
                        violations.append("last_seen_source_citation_missing")
        if last_seen_ok:
            last_seen_correct+=1

        violations=sorted(set(violations))
        violations_total+=len(violations)
        provenance_complete=provenance_complete and not violations
        qualified=answer_ok and provenance_complete
        provenance_count+=int(provenance_complete)
        qualified_count+=int(qualified)
        instance_local_truth={k[1:]:v for k,v in all_instance_truth.items() if k[0]==episode_id}
        instance_local_pred={k[1:]:v for k,v in all_instance_predictions.items() if k[0]==episode_id}
        room_local_truth={k[1:]:v for k,v in all_room_truth.items() if k[0]==episode_id}
        room_local_pred={k[1:]:v for k,v in all_room_predictions.items() if k[0]==episode_id}
        instance_result=_confusion(instance_local_truth,instance_local_pred)
        room_result=_confusion(room_local_truth,room_local_pred)
        episode_results.append({
            "episode_id":episode_id,"answer_correct":answer_ok,"decision_correct":decision_ok,
            "citation_precision":citation_precision,"citation_recall":citation_recall,
            "provenance_complete":provenance_complete,
            "instance_pair_accuracy":instance_result["accuracy"],
            "false_merge_count":instance_result["false_merge_count"],
            "false_split_count":instance_result["false_split_count"],
            "room_pair_accuracy":room_result["accuracy"],
            "last_seen_correct":last_seen_ok,
            "evidence_qualified_answer_correct":qualified,"violations":violations
        })

    instance_metrics=_confusion(all_instance_truth,all_instance_predictions)
    room_metrics=_confusion(all_room_truth,all_room_predictions)
    total=len(ids)
    return {
        "schema_version":SCHEMA_VERSION,"benchmark_id":BENCHMARK_ID,
        "policy_id":submission.get("policy_id"),"run_id":submission.get("run_id"),
        "interpretation":{
            "status":"synthetic_spatial_identity_reference_score",
            "limitations":[
                "All observations contain synthetic metadata and placeholder digests; no source image, mask, biometric or physical room map is present.",
                "Stable instance and room identities exist only in evaluator truth and are not candidate inputs.",
                "Candidate declarations are not evidence of true visual recognition or internal model attention.",
                "This finite authored policy validates evaluator semantics only; it is not an EgoWitness score or SOTA comparison.",
                "The external EgoWitness release, media rights and scorer revision must be pinned before any real dataset reproduction."
            ]
        },
        "totals":{
            "episode_count":total,"answer_correct_count":answer_correct,
            "answer_accuracy":ratio(answer_correct,total),
            "decision_correct_count":decision_correct,"decision_accuracy":ratio(decision_correct,total),
            "coverage":ratio(coverage,total),
            "selective_accuracy":ratio(answer_correct,coverage),
            "citation_precision":ratio(citation_tp,citation_pred),
            "citation_recall":ratio(citation_tp,citation_required),
            "provenance_complete_count":provenance_count,
            "provenance_complete_rate":ratio(provenance_count,total),
            "instance_pair_count":instance_metrics["count"],
            "instance_pair_correct_count":instance_metrics["correct"],
            "instance_pair_accuracy":instance_metrics["accuracy"],
            "instance_pair_precision":instance_metrics["precision"],
            "instance_pair_recall":instance_metrics["recall"],
            "false_merge_count":instance_metrics["false_merge_count"],
            "false_split_count":instance_metrics["false_split_count"],
            "room_pair_count":room_metrics["count"],
            "room_pair_correct_count":room_metrics["correct"],
            "room_pair_accuracy":room_metrics["accuracy"],
            "last_seen_count":last_seen_count,
            "last_seen_correct_count":last_seen_correct,
            "last_seen_accuracy":ratio(last_seen_correct,last_seen_count),
            "false_wearer_attribution_count":false_wearer,
            "evidence_qualified_answer_correct_count":qualified_count,
            "evidence_qualified_answer_accuracy":ratio(qualified_count,total),
            "hard_violation_count":violations_total
        },
        "per_episode":episode_results,
        "hard_violations":[{"episode_id":row["episode_id"],"violations":row["violations"]} for row in episode_results if row["violations"]]
    }


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload",required=True);parser.add_argument("--truth",required=True)
    parser.add_argument("--submission",required=True);parser.add_argument("--output",required=True)
    args=parser.parse_args()
    try:
        score=evaluate(read_json(args.workload),read_json(args.truth),read_json(args.submission))
        out=Path(args.output);out.parent.mkdir(parents=True,exist_ok=True)
        out.write_text(json.dumps(score,indent=2,sort_keys=True)+"\n",encoding="utf-8")
    except (OSError,json.JSONDecodeError,EvaluationError) as error:
        print(f"EgoWitness evaluation failed: {error}",file=sys.stderr);return 2
    print(f"wrote {out}; episodes={score['totals']['episode_count']} violations={score['totals']['hard_violation_count']}")
    return 0


if __name__=="__main__":
    raise SystemExit(main())